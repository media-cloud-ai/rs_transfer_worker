use crate::target_configuration::TargetConfiguration;

use ftp::FtpError;

use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Read};

pub struct FileReader {
  target: TargetConfiguration,
}

impl FileReader {
  pub fn new(target: TargetConfiguration) -> Self {
    FileReader { target }
  }

  pub fn process_copy<F>(&mut self, streamer: F) -> Result<(), FtpError>
  where
    F: Fn(&mut dyn Read) -> Result<(), FtpError>,
  {
    let source_file = File::open(&self.target.path)
      .map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::Other, e.to_string())))?;

    let mut data_stream = BufReader::new(source_file);
    streamer(&mut data_stream)
  }
}
