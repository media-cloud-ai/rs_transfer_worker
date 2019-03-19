use amqp_worker::job::Job;
use amqp_worker::MessageError;

use ftp::openssl::ssl::{SslContext, SslMethod};
use ftp::types::FileType;
use ftp::FtpError;
use ftp::FtpStream;

use std::io::{Error, ErrorKind};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct TargetConfiguration {
  hostname: Option<String>,
  port: u16,
  username: Option<String>,
  password: Option<String>,
  pub prefix: Option<String>,
  pub path: String,
  ssl_enabled: bool,
}

impl TargetConfiguration {
  pub fn new(job: &Job, target: &str) -> Result<Self, MessageError> {
    let path_parameter = format!("{}_path", target);

    let hostname = job
      .get_credential_parameter(&format!("{}_hostname", target))
      .map(|key| key.request_value(&job))
      .map_or(Ok(None), |r| r.map(Some))?;

    let password = job
      .get_credential_parameter(&format!("{}_password", target))
      .map(|key| key.request_value(&job))
      .map_or(Ok(None), |r| r.map(Some))?;

    let username = job
      .get_credential_parameter(&format!("{}_username", target))
      .map(|key| key.request_value(&job))
      .map_or(Ok(None), |r| r.map(Some))?;

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
          MessageError::ProcessingError(job.job_id, format!("unable to parse port value: {}", e))
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
          MessageError::ProcessingError(job.job_id, format!("unable to parse ssl enabling: {}", e))
        })
      })
      .map_or(Ok(None), |r| r.map(Some))?
      .unwrap_or(false);

    let path = job.get_string_parameter(&path_parameter).ok_or_else(|| {
      MessageError::ProcessingError(
        job.job_id,
        format!("missing {} parameter", path_parameter.replace("_", " ")),
      )
    })?;

    Ok(TargetConfiguration {
      hostname,
      password,
      path,
      port,
      prefix,
      ssl_enabled,
      username,
    })
  }

  pub fn is_ftp_configured(&self) -> bool {
    self.hostname.is_some()
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
}
