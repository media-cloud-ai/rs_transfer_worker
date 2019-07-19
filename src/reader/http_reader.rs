use crate::target_configuration::TargetConfiguration;

use ftp::FtpError;
use reqwest;
use reqwest::StatusCode;
use std::io::{Error, ErrorKind, Read};
use std::thread;

pub struct HttpReader {
  target: TargetConfiguration,
}

impl HttpReader {
  pub fn new(target: TargetConfiguration) -> Self {
    HttpReader { target }
  }

  pub fn process_copy<F: 'static + Sync + Send>(&mut self, streamer: F) -> Result<(), FtpError>
  where
    F: Fn(&mut dyn Read) -> Result<(), FtpError>,
  {
    let url = self.target.path.clone();
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
        println!("ERROR {:?}", response);
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
