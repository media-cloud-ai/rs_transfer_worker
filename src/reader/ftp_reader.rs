use crate::target_configuration::TargetConfiguration;

use ftp::FtpError;
use std::io::Read;
use std::path::Path;

pub struct FtpReader {
  target: TargetConfiguration,
}

impl FtpReader {
  pub fn new(target: TargetConfiguration) -> Self {
    FtpReader { target }
  }

  pub fn process_copy<F>(&mut self, streamer: F) -> Result<(), FtpError>
  where
    F: Fn(&mut dyn Read) -> Result<(), FtpError>,
  {
    let prefix = self
      .target
      .prefix
      .clone()
      .unwrap_or_else(|| "/".to_string());
    let absolute_path = prefix + &self.target.path;

    let path = Path::new(&absolute_path);
    let directory = path.parent().unwrap().to_str().unwrap();
    let filename = path.file_name().unwrap().to_str().unwrap();

    let mut ftp_stream = self.target.get_ftp_stream()?;
    ftp_stream.cwd(&directory)?;
    ftp_stream.retr(&filename, streamer)
  }
}
