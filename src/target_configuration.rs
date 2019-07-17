use amqp_worker::job::*;
use amqp_worker::MessageError;

use ftp::openssl::ssl::{SslContext, SslMethod};
use ftp::types::FileType;
use ftp::FtpError;
use ftp::FtpStream;

use rusoto_core::request::HttpClient;
use rusoto_core::region::Region;
use rusoto_credential::StaticProvider;
use rusoto_s3::{
  GetObjectRequest,
  S3Client,
  S3,
};

use std::io::{Error, ErrorKind};
use std::str::FromStr;

#[derive(Debug)]
pub enum ConfigurationType {
  LocalFile,
  Ftp,
  S3Bucket
}

#[derive(Debug, Clone)]
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
      .map_or(Ok(Region::default()), |r| Region::from_str(&r.unwrap())
        .map_err(|e| {
          let result = JobResult::new(job.job_id, JobStatus::Error, vec![])
            .with_message(format!("unable to match AWS region: {}", e));
          MessageError::ProcessingError(result)
        }))?;

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

    let path = job.get_string_parameter(&path_parameter).ok_or_else(|| {
      let result = JobResult::new(job.job_id, JobStatus::Error, vec![])
        .with_message(format!("missing {} parameter", path_parameter.replace("_", " ")));
      MessageError::ProcessingError(result)
    })?;

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

  #[cfg(test)]
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

  #[cfg(test)]
  pub fn new_ftp(hostname: &str, username: &str, password: &str, prefix: &str, path: &str) -> Self {
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
      ssl_enabled: false,
    }
  }

  #[cfg(test)]
  pub fn new_s3(access_key: &str, secret_key: &str, region: Region, prefix: &str, path: &str) -> Self {
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

  pub fn get_type(&self) -> ConfigurationType {
    if self.hostname.is_some() {
      return ConfigurationType::Ftp;
    }
    if self.secret_key.is_some() {
      return ConfigurationType::S3Bucket;
    }
    ConfigurationType::LocalFile
  }

  pub fn get_ftp_stream(&self) -> Result<FtpStream, FtpError> {
    let mut ftp_stream = FtpStream::connect((self.hostname.clone().unwrap().as_str(), self.port))?;
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

  pub fn get_s3_stream(&self) -> Result<rusoto_core::ByteStream, FtpError> {
    let client = S3Client::new_with(
      HttpClient::new().expect("Unable to create HTTP client"),
      StaticProvider::new_minimal(self.access_key.clone().unwrap(), self.secret_key.clone().unwrap()),
      self.region.clone()
    );

    let request = GetObjectRequest {
      bucket: self.prefix.clone().unwrap(),
      key: self.path.clone(),
      ..Default::default()
    };

    let object = client.get_object(request).sync().unwrap();
    let stream = object.body.unwrap();
    Ok(stream)
  }
}
