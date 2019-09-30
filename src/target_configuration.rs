use std::io::{Error, ErrorKind};
use std::str::FromStr;

use amqp_worker::{job::*, MessageError};
use ftp::{
  openssl::ssl::{SslContext, SslMethod},
  types::FileType,
  FtpError, FtpStream,
};
use rusoto_core::{region::Region, request::HttpClient};
use rusoto_credential::StaticProvider;
use rusoto_s3::{
  CompleteMultipartUploadRequest, CompletedMultipartUpload, CompletedPart,
  CreateMultipartUploadRequest, GetObjectRequest, S3Client, UploadPartRequest, S3,
};
use url::Url;

#[derive(Debug, PartialEq)]
pub enum ConfigurationType {
  Ftp,
  HttpResource,
  LocalFile,
  S3Bucket,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TargetConfiguration {
  hostname: Option<String>,
  port: u16,
  username: Option<String>,
  password: Option<String>,
  access_key: Option<String>,
  secret_key: Option<String>,
  region: Region,
  pub prefix: Option<String>,
  pub path: String,
  ssl_enabled: bool,
}

impl TargetConfiguration {
  pub fn new(job: &Job, target: &str) -> Result<Self, MessageError> {
    let path_parameter = format!("{}_path", target);

    let path = job.get_string_parameter(&path_parameter).ok_or_else(|| {
      let result = JobResult::new(job.job_id, JobStatus::Error, vec![]).with_message(format!(
        "missing {} parameter",
        path_parameter.replace("_", " ")
      ));
      MessageError::ProcessingError(result)
    })?;

    if let Ok(url) = Url::parse(path.as_str()) {
      return TargetConfiguration::get_target_from_url(job, &url);
    }

    let hostname = job
      .get_credential_parameter(&format!("{}_hostname", target))
      .map(|key| key.request_value(job))
      .map_or(Ok(None), |r| r.map(Some))?;

    let password = job
      .get_credential_parameter(&format!("{}_password", target))
      .map(|key| key.request_value(job))
      .map_or(Ok(None), |r| r.map(Some))?;

    let username = job
      .get_credential_parameter(&format!("{}_username", target))
      .map(|key| key.request_value(job))
      .map_or(Ok(None), |r| r.map(Some))?;

    let access_key = job
      .get_credential_parameter(&format!("{}_access_key", target))
      .map(|key| key.request_value(job))
      .map_or(Ok(None), |r| r.map(Some))?;

    let secret_key = job
      .get_credential_parameter(&format!("{}_secret_key", target))
      .map(|key| key.request_value(job))
      .map_or(Ok(None), |r| r.map(Some))?;

    let region = job
      .get_credential_parameter(&format!("{}_region", target))
      .map(|key| key.request_value(job))
      .map_or(Ok(Region::default()), |r| {
        if let Some(h) = &hostname {
          Ok(Region::Custom {
            name: r.unwrap(),
            endpoint: h.clone(),
          })
        } else {
          Region::from_str(&r.unwrap()).map_err(|e| {
            let result = JobResult::new(job.job_id, JobStatus::Error, vec![])
              .with_message(format!("unable to match AWS region: {}", e));
            MessageError::ProcessingError(result)
          })
        }
      })?;

    let prefix = job
      .get_credential_parameter(&format!("{}_prefix", target))
      .map(|key| key.request_value(&job))
      .map_or(Ok(None), |r| r.map(Some))?;

    let port = job
      .get_credential_parameter(&format!("{}_port", target))
      .map(|key| key.request_value(&job))
      .map_or(Ok(None), |r| r.map(Some))?
      .map(|value| {
        value.parse::<u16>().map_err(|e| {
          let result = JobResult::new(job.job_id, JobStatus::Error, vec![])
            .with_message(format!("unable to parse port value: {}", e));
          MessageError::ProcessingError(result)
        })
      })
      .map_or(Ok(None), |r| r.map(Some))?
      .unwrap_or(21);

    let ssl_enabled = job
      .get_credential_parameter(&format!("{}_ssl", target))
      .map(|key| key.request_value(&job))
      .map_or(Ok(None), |r| r.map(Some))?
      .map(|value| {
        FromStr::from_str(&value).map_err(|e| {
          let result = JobResult::new(job.job_id, JobStatus::Error, vec![])
            .with_message(format!("unable to parse ssl enabling: {}", e));
          MessageError::ProcessingError(result)
        })
      })
      .map_or(Ok(None), |r| r.map(Some))?
      .unwrap_or(false);

    Ok(TargetConfiguration {
      access_key,
      hostname,
      password,
      path,
      port,
      prefix,
      region,
      secret_key,
      ssl_enabled,
      username,
    })
  }

  pub fn new_file(path: &str) -> Self {
    TargetConfiguration {
      hostname: None,
      port: 0,
      username: None,
      password: None,
      access_key: None,
      secret_key: None,
      region: Region::default(),
      prefix: None,
      path: path.to_string(),
      ssl_enabled: false,
    }
  }

  pub fn new_ftp(hostname: &str, username: &str, password: &str, prefix: &str, path: &str) -> Self {
    TargetConfiguration::new_ftp_with_ssl(hostname, username, password, prefix, path, false)
  }

  pub fn new_ftp_with_ssl(
    hostname: &str,
    username: &str,
    password: &str,
    prefix: &str,
    path: &str,
    ssl_enabled: bool,
  ) -> Self {
    TargetConfiguration {
      hostname: Some(hostname.to_string()),
      port: 21,
      username: Some(username.to_string()),
      password: Some(password.to_string()),
      access_key: None,
      secret_key: None,
      region: Region::default(),
      prefix: Some(prefix.to_string()),
      path: path.to_string(),
      ssl_enabled,
    }
  }

  pub fn new_s3(
    access_key: &str,
    secret_key: &str,
    region: Region,
    prefix: &str,
    path: &str,
  ) -> Self {
    TargetConfiguration {
      hostname: None,
      port: 0,
      username: None,
      password: None,
      access_key: Some(access_key.to_string()),
      secret_key: Some(secret_key.to_string()),
      region,
      prefix: Some(prefix.to_string()),
      path: path.to_string(),
      ssl_enabled: false,
    }
  }

  pub fn new_http(path: &str) -> Self {
    TargetConfiguration::new_http_with_ssl(path, false)
  }

  pub fn new_http_with_ssl(path: &str, ssl_enabled: bool) -> Self {
    TargetConfiguration {
      hostname: None,
      port: 0,
      username: None,
      password: None,
      access_key: None,
      secret_key: None,
      region: Region::default(),
      prefix: None,
      path: path.to_string(),
      ssl_enabled,
    }
  }

  fn get_target_from_url(job: &Job, url: &Url) -> Result<TargetConfiguration, MessageError> {
    match url.scheme() {
      "file" => Ok(TargetConfiguration::new_file(url.as_str())),
      "http" => Ok(TargetConfiguration::new_http(url.as_str())),
      "https" => Ok(TargetConfiguration::new_http_with_ssl(url.as_str(), true)),
      "ftp" => {
        if let (Some(hostname), Some(password)) = (url.host_str(), url.password()) {
          Ok(TargetConfiguration::new_ftp(
            hostname,
            url.username(),
            password,
            "",
            url.path(),
          ))
        } else {
          let result = JobResult::new(job.job_id, JobStatus::Error, vec![])
            .with_message(format!("Invalid FTP URL: {}", url));
          Err(MessageError::ProcessingError(result))
        }
      }
      "sftp" => {
        if let (Some(hostname), Some(password)) = (url.host_str(), url.password()) {
          Ok(TargetConfiguration::new_ftp_with_ssl(
            hostname,
            url.username(),
            password,
            "",
            url.path(),
            true,
          ))
        } else {
          let result = JobResult::new(job.job_id, JobStatus::Error, vec![])
            .with_message(format!("Invalid SFTP URL: {}", url));
          Err(MessageError::ProcessingError(result))
        }
      }
      "s3" => {
        let region_str = TargetConfiguration::get_value_from_url_parameters(job, url, "region")?;

        let region = if let Ok(hostname_str) =
          TargetConfiguration::get_value_from_url_parameters(job, url, "hostname")
        {
          Region::Custom {
            name: region_str,
            endpoint: hostname_str,
          }
        } else {
          Region::from_str(region_str.as_str()).map_err(|error| {
            let result =
              JobResult::new(job.job_id, JobStatus::Error, vec![]).with_message(error.to_string());
            MessageError::ProcessingError(result)
          })?
        };

        let access_key =
          TargetConfiguration::get_value_from_url_parameters(job, url, "access_key")?;
        let secret_key =
          TargetConfiguration::get_value_from_url_parameters(job, url, "secret_key")?;

        if let Some(hostname) = url.host_str() {
          Ok(TargetConfiguration::new_s3(
            access_key.as_str(),
            secret_key.as_str(),
            region,
            hostname,
            url.path(),
          ))
        } else {
          let result = JobResult::new(job.job_id, JobStatus::Error, vec![])
            .with_message(format!("Invalid S3 URL: {}", url));
          Err(MessageError::ProcessingError(result))
        }
      }
      _ => {
        let result = JobResult::new(job.job_id, JobStatus::Error, vec![])
          .with_message(format!("Unsupported URL: {}", url));
        Err(MessageError::ProcessingError(result))
      }
    }
  }

  fn get_value_from_url_parameters(
    job: &Job,
    url: &Url,
    reference_key: &str,
  ) -> Result<String, MessageError> {
    url
      .query_pairs()
      .into_owned()
      .find(|(key, _value)| {
        key.eq(reference_key) || key.eq(&("credential_".to_string() + reference_key))
      })
      .map(|(key, value)| {
        if key.starts_with("credential_") {
          let credential = Credential { key: value };
          credential.request_value(job)
        } else {
          Ok(value)
        }
      })
      .unwrap_or({
        let result = JobResult::new(job.job_id, JobStatus::Error, vec![])
          .with_message(format!("Cannot find {:?} into url: {}", reference_key, url));
        Err(MessageError::ProcessingError(result))
      })
  }

  pub fn get_type(&self) -> ConfigurationType {
    if self.hostname.is_some() {
      return ConfigurationType::Ftp;
    }
    if self.secret_key.is_some() {
      return ConfigurationType::S3Bucket;
    }

    if self.path.starts_with("http://") || self.path.starts_with("https://") {
      ConfigurationType::HttpResource
    } else {
      ConfigurationType::LocalFile
    }
  }

  pub fn get_ftp_stream(&self) -> Result<FtpStream, FtpError> {
    let hostname = if let Some(hostname) = &self.hostname {
      hostname
    } else {
      return Err(FtpError::InvalidResponse(
        "Missing hostname to access to FTP content".to_string(),
      ));
    };

    let mut ftp_stream = FtpStream::connect((hostname.as_str(), self.port))?;
    if self.ssl_enabled {
      let builder = SslContext::builder(SslMethod::tls()).map_err(|_e| {
        FtpError::ConnectionError(Error::new(ErrorKind::Other, "unable to build SSL context"))
      })?;
      let context = builder.build();
      // Switch to secure mode
      ftp_stream = ftp_stream.into_secure(context)?;
    }

    if let (Some(username), Some(password)) = (self.username.clone(), self.password.clone()) {
      ftp_stream.login(username.as_str(), password.as_str())?;
    }

    ftp_stream.transfer_type(FileType::Binary)?;
    Ok(ftp_stream)
  }

  pub fn get_s3_download_stream(&self) -> Result<rusoto_core::ByteStream, FtpError> {
    let request = GetObjectRequest {
      bucket: self.get_s3_bucket()?,
      key: self.path.clone(),
      ..Default::default()
    };

    let object = self
      .get_s3_client()?
      .get_object(request)
      .sync()
      .map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::ConnectionRefused, e)))?;
    let stream = object.body.ok_or(FtpError::InvalidResponse(
      "No retrieved object data to access.".to_string(),
    ))?;
    Ok(stream)
  }

  pub fn start_multi_part_s3_upload(&self) -> Result<String, FtpError> {
    let request = CreateMultipartUploadRequest {
      bucket: self.get_s3_bucket()?,
      key: self.path.clone(),
      ..Default::default()
    };

    let object = self
      .get_s3_client()?
      .create_multipart_upload(request)
      .sync()
      .map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::ConnectionRefused, e)))?;

    if let Some(upload_id) = object.upload_id {
      Ok(upload_id)
    } else {
      Err(FtpError::ConnectionError(Error::new(
        ErrorKind::ConnectionRefused,
        "error",
      )))
    }
  }

  pub fn upload_s3_part(
    &self,
    upload_id: &str,
    part_number: i64,
    data: Vec<u8>,
  ) -> Result<CompletedPart, FtpError> {
    let request = UploadPartRequest {
      body: Some(rusoto_core::ByteStream::from(data)),
      bucket: self.get_s3_bucket()?,
      key: self.path.clone(),
      upload_id: upload_id.to_string(),
      part_number,
      ..Default::default()
    };

    let object = self
      .get_s3_client()?
      .upload_part(request)
      .sync()
      .map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::ConnectionRefused, e)))?;

    Ok(CompletedPart {
      e_tag: object.e_tag.clone(),
      part_number: Some(part_number),
    })
  }

  pub fn complete_s3_upload(
    &self,
    upload_id: String,
    parts: Vec<CompletedPart>,
  ) -> Result<(), FtpError> {
    let request = CompleteMultipartUploadRequest {
      bucket: self.get_s3_bucket()?,
      key: self.path.clone(),
      upload_id,
      multipart_upload: Some(CompletedMultipartUpload { parts: Some(parts) }),
      ..Default::default()
    };

    self
      .get_s3_client()?
      .complete_multipart_upload(request)
      .sync()
      .map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::ConnectionRefused, e)))?;

    Ok(())
  }

  fn get_s3_client(&self) -> Result<S3Client, FtpError> {
    let access_key = if let Some(access_key) = &self.access_key {
      access_key.to_string()
    } else {
      return Err(FtpError::InvalidResponse(
        "Missing access_key to access to S3 content".to_string(),
      ));
    };

    let secret_key = if let Some(secret_key) = &self.secret_key {
      secret_key.to_string()
    } else {
      return Err(FtpError::InvalidResponse(
        "Missing secret_key to access to S3 content".to_string(),
      ));
    };

    Ok(S3Client::new_with(
      HttpClient::new()
        .map_err(|_| FtpError::InvalidResponse("Unable to create HTTP client".to_string()))?,
      StaticProvider::new_minimal(access_key, secret_key),
      self.region.clone(),
    ))
  }

  fn get_s3_bucket(&self) -> Result<String, FtpError> {
    if let Some(prefix) = &self.prefix {
      Ok(prefix.to_string())
    } else {
      Err(FtpError::InvalidResponse(
        "Missing prefix (used as bucket identifier) to access to S3 content".to_string(),
      ))
    }
  }
}

#[test]
pub fn get_value_from_url_parameters_test() {
  let message = r#"
    {
      "job_id": 123,
      "parameters": [
      ]
    }
  "#;
  let job = Job::new(message).unwrap();

  let url1 = Url::parse("https://www.google.com").unwrap();
  let result1 = TargetConfiguration::get_value_from_url_parameters(&job, &url1, "search");
  assert!(result1.is_err());

  let url2 = Url::parse("https://www.google.com?search=hello").unwrap();
  let result2 = TargetConfiguration::get_value_from_url_parameters(&job, &url2, "search");
  assert!(result2.is_ok());
  assert_eq!("hello", result2.unwrap().as_str());

  let url3 = Url::parse("https://www.google.com?page=23&search=hello").unwrap();
  let result3 = TargetConfiguration::get_value_from_url_parameters(&job, &url3, "search");
  assert!(result3.is_ok());
  assert_eq!("hello", result3.unwrap().as_str());

  let result4 = TargetConfiguration::get_value_from_url_parameters(&job, &url3, "page");
  assert!(result4.is_ok());
  assert_eq!("23", result4.unwrap().as_str());
}

#[cfg(test)]
fn validate_target(
  result: Result<TargetConfiguration, MessageError>,
  path: &str,
  ssl_enabled: bool,
) {
  assert!(result.is_ok());
  let target = result.unwrap();
  assert_eq!(path, target.path);
  assert_eq!(ssl_enabled, target.ssl_enabled);
  assert_eq!(None, target.hostname);
  assert_eq!(0, target.port);
  assert_eq!(None, target.username);
  assert_eq!(None, target.password);
  assert_eq!(None, target.access_key);
  assert_eq!(None, target.secret_key);
  assert_eq!(Region::default(), target.region);
  assert_eq!(None, target.prefix);
}

#[test]
pub fn get_target_from_url_test_file() {
  let message = r#"
    {
      "job_id": 123,
      "parameters": [
      ]
    }
  "#;
  let job = Job::new(message).unwrap();

  let path = "file://path/to/local/file";
  let url = Url::parse(path).unwrap();
  let result = TargetConfiguration::get_target_from_url(&job, &url);
  validate_target(result, path, false);
}

#[test]
pub fn get_target_from_url_test_http() {
  let message = r#"
    {
      "job_id": 123,
      "parameters": [
      ]
    }
  "#;
  let job = Job::new(message).unwrap();

  let path = "http://www.google.com/";
  let url = Url::parse(path).unwrap();
  let result = TargetConfiguration::get_target_from_url(&job, &url);
  validate_target(result, path, false);
}

#[test]
pub fn get_target_from_url_test_https() {
  let message = r#"
    {
      "job_id": 123,
      "parameters": [
      ]
    }
  "#;
  let job = Job::new(message).unwrap();

  let path = "https://www.google.com/";
  let url = Url::parse(path).unwrap();
  let result = TargetConfiguration::get_target_from_url(&job, &url);
  validate_target(result, path, true);
}

#[test]
pub fn get_target_from_url_test_ftp() {
  let message = r#"
    {
      "job_id": 123,
      "parameters": [
      ]
    }
  "#;
  let job = Job::new(message).unwrap();

  let path = "ftp://username:password@hostname/folder/file";
  let url = Url::parse(path).unwrap();
  let result = TargetConfiguration::get_target_from_url(&job, &url);
  assert!(result.is_ok());
  let target = result.unwrap();
  assert_eq!(Some("hostname".to_string()), target.hostname);
  assert_eq!(21, target.port);
  assert_eq!(Some("username".to_string()), target.username);
  assert_eq!(Some("password".to_string()), target.password);
  assert_eq!(None, target.access_key);
  assert_eq!(None, target.secret_key);
  assert_eq!(Region::default(), target.region);
  assert_eq!(Some("".to_string()), target.prefix);
  assert_eq!("/folder/file".to_string(), target.path);
  assert_eq!(false, target.ssl_enabled);
}

#[test]
pub fn get_target_from_url_test_sftp() {
  let message = r#"
    {
      "job_id": 123,
      "parameters": [
      ]
    }
  "#;
  let job = Job::new(message).unwrap();

  let path = "sftp://username:password@hostname/folder/file";
  let url = Url::parse(path).unwrap();
  let result = TargetConfiguration::get_target_from_url(&job, &url);
  assert!(result.is_ok());
  let target = result.unwrap();
  assert_eq!(Some("hostname".to_string()), target.hostname);
  assert_eq!(21, target.port);
  assert_eq!(Some("username".to_string()), target.username);
  assert_eq!(Some("password".to_string()), target.password);
  assert_eq!(None, target.access_key);
  assert_eq!(None, target.secret_key);
  assert_eq!(Region::default(), target.region);
  assert_eq!(Some("".to_string()), target.prefix);
  assert_eq!("/folder/file".to_string(), target.path);
  assert_eq!(true, target.ssl_enabled);
}

#[test]
pub fn get_target_from_url_test_s3() {
  let message = r#"
    {
      "job_id": 123,
      "parameters": [
      ]
    }
  "#;
  let job = Job::new(message).unwrap();

  let path =
    "s3://bucket/folder/file?region=eu-central-1&access_key=login&secret_key=password&hostname=hostname";
  let url = Url::parse(path).unwrap();
  let result = TargetConfiguration::get_target_from_url(&job, &url);
  assert!(result.is_ok());
  let target = result.unwrap();
  assert_eq!(None, target.hostname);
  assert_eq!(0, target.port);
  assert_eq!(None, target.username);
  assert_eq!(None, target.password);
  assert_eq!(Some("login".to_string()), target.access_key);
  assert_eq!(Some("password".to_string()), target.secret_key);
  assert_eq!(
    Region::Custom {
      name: "eu-central-1".to_string(),
      endpoint: "hostname".to_string()
    },
    target.region
  );
  assert_eq!(Some("bucket".to_string()), target.prefix);
  assert_eq!("/folder/file".to_string(), target.path);
  assert_eq!(false, target.ssl_enabled);
}

#[test]
pub fn get_target_from_url_test_s3_with_credentials() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/MEDIAIO_AWS_ACCESS_KEY")
    .with_header("content-type", "application/json")
    .with_body(
      r#"{"data": {
      "id": 666,
      "key": "MEDIAIO_AWS_ACCESS_KEY",
      "value": "AKAIMEDIAIO",
      "inserted_at": "today"
    }}"#,
    )
    .create();

  let _m = mock("GET", "/credentials/MEDIAIO_AWS_SECRET_KEY")
    .with_header("content-type", "application/json")
    .with_body(
      r#"{"data": {
      "id": 666,
      "key": "MEDIAIO_AWS_SECRET_KEY",
      "value": "SECRETKEYFORMEDIAIO",
      "inserted_at": "today"
    }}"#,
    )
    .create();

  let message = r#"
    {
      "job_id": 123,
      "parameters": [
      ]
    }
  "#;
  let job = Job::new(message).unwrap();

  let path = "s3://bucket/folder/file?region=eu-central-1&credential_access_key=MEDIAIO_AWS_ACCESS_KEY&credential_secret_key=MEDIAIO_AWS_SECRET_KEY&hostname=hostname";
  let url = Url::parse(path).unwrap();
  let result = TargetConfiguration::get_target_from_url(&job, &url);
  assert!(result.is_ok());
  let target = result.unwrap();
  assert_eq!(None, target.hostname);
  assert_eq!(0, target.port);
  assert_eq!(None, target.username);
  assert_eq!(None, target.password);
  assert_eq!(Some("AKAIMEDIAIO".to_string()), target.access_key);
  assert_eq!(Some("SECRETKEYFORMEDIAIO".to_string()), target.secret_key);
  assert_eq!(
    Region::Custom {
      name: "eu-central-1".to_string(),
      endpoint: "hostname".to_string()
    },
    target.region
  );
  assert_eq!(Some("bucket".to_string()), target.prefix);
  assert_eq!("/folder/file".to_string(), target.path);
  assert_eq!(false, target.ssl_enabled);
}

#[test]
pub fn new_target_from_url_test() {
  let message = r#"
    {
      "job_id": 123,
      "parameters": [
        {
          "id": "source_path",
          "type": "string",
          "value": "ftp://username:password@hostname/folder/file"
        }
      ]
    }
  "#;
  let job = Job::new(message).unwrap();
  let target = TargetConfiguration::new(&job, "source").unwrap();
  assert_eq!(ConfigurationType::Ftp, target.get_type());
  assert_eq!(Some("hostname".to_string()), target.hostname);
  assert_eq!(21, target.port);
  assert_eq!(Some("username".to_string()), target.username);
  assert_eq!(Some("password".to_string()), target.password);
  assert_eq!(None, target.access_key);
  assert_eq!(None, target.secret_key);
  assert_eq!(Region::default(), target.region);
  assert_eq!(Some("".to_string()), target.prefix);
  assert_eq!("/folder/file".to_string(), target.path);
  assert_eq!(false, target.ssl_enabled);
}

#[test]
pub fn new_target_from_non_url_test() {
  use mockito::mock;
  use rusoto_core::Region::UsEast1;

  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/SOME_HOST_CREDENTIAL")
    .with_header("content-type", "application/json")
    .with_body(
      r#"{"data": {
      "id": 666,
      "key": "SOME_HOST_CREDENTIAL",
      "value": "https://s3.media-io.com",
      "inserted_at": "today"
    }}"#,
    )
    .create();

  let message = r#"
    {
      "job_id": 123,
      "parameters": [
        {
          "id": "source_path",
          "type": "string",
          "value": "/path/to/file"
        },
        {
          "id": "source_hostname",
          "type": "credential",
          "value": "SOME_HOST_CREDENTIAL"
        }
      ]
    }
  "#;
  let job = Job::new(message).unwrap();
  let result = TargetConfiguration::new(&job, "source");
  assert!(result.is_ok());
  let target = result.unwrap();
  assert_eq!(
    target,
    TargetConfiguration {
      hostname: Some("https://s3.media-io.com".to_string()),
      port: 21,
      username: None,
      password: None,
      access_key: None,
      secret_key: None,
      region: UsEast1,
      prefix: None,
      path: "/path/to/file".to_string(),
      ssl_enabled: false
    }
  );
}
