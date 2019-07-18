
use crate::target_configuration::TargetConfiguration;
use crate::writer::StreamWriter;

use ftp::FtpError;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{copy, BufWriter, Error, ErrorKind, Read};
use std::path::Path;

#[derive(Debug)]
pub struct FileStreamWriter {
  target: TargetConfiguration,
}

impl FileStreamWriter {
  pub fn new(target: TargetConfiguration) -> Self {
    FileStreamWriter { target }
  }

  pub fn open(&mut self) -> Result<(), FtpError> {
    let destination_path = Path::new(self.target.path.as_str());
    let destination_directory = destination_path.parent().unwrap_or_else(|| Path::new("/"));
    fs::create_dir_all(destination_directory).map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::Other, e.to_string())))?;

    let _destination_file = File::create(&self.target.path)
      .map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::Other, e.to_string())))?;
    Ok(())
  }
}

impl StreamWriter for FileStreamWriter {
  fn write_stream(&self, read_stream: &mut dyn Read) -> Result<(), FtpError> {
    let destination_file =
      OpenOptions::new()
      .write(true)
      .create(false)
      .append(true)
      .open(&self.target.path)
      .map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::Other, e.to_string())))?;

    let mut file_writer: BufWriter<File> = BufWriter::new(destination_file);
    copy(read_stream, &mut file_writer).map_err(|e| FtpError::ConnectionError(e))?;
    Ok(())
  }
}
