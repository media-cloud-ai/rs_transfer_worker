use crate::target_configuration::TargetConfiguration;

use amqp_worker::job::Job;
use ftp::FtpError;
use reqwest;
use reqwest::StatusCode;
use std::io::{Error, ErrorKind, Read};
use std::thread;

pub struct HttpReader {
  job_id: u64,
  target: TargetConfiguration,
}

impl HttpReader {
  pub fn new(target: TargetConfiguration, job: &Job) -> Self {
    HttpReader {
      job_id: job.job_id,
      target,
    }
  }

  pub fn process_copy<F: 'static + Sync + Send>(&mut self, streamer: F) -> Result<(), FtpError>
  where
    F: Fn(&mut dyn Read) -> Result<(), FtpError>,
  {
    let url = self.target.path.clone();
    let job_id = self.job_id;

    let request_thread = thread::spawn(move || {
      let client = reqwest::Client::builder()
        .build()
        .map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::Other, e.to_string())))?;

      let mut response = client
        .get(url.as_str())
        .send()
        .map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::Other, e.to_string())))?;

      let status = response.status();

      if status != StatusCode::OK {
        error!(target: &job_id.to_string(), "{:?}", response);
        return Err(FtpError::ConnectionError(Error::new(
          ErrorKind::Other,
          "bad response status".to_string(),
        )));
      }

      streamer(&mut response)
    });

    request_thread
      .join()
      .map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::Other, format!("{:?}", e))))?
  }
}
