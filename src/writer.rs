use crate::target_configuration::TargetConfiguration;

use ftp::FtpError;
use std::fs::File;
use std::io::{copy, BufReader, BufWriter, Error, ErrorKind, Read};
use std::path::{Path, PathBuf};

pub trait StreamWriter: Sized {
  fn write_stream(&self, reader: &mut Read) -> Result<(), FtpError>;
}

#[derive(Debug)]
pub struct FileStreamWriter {
  target: TargetConfiguration,
}

impl FileStreamWriter {
  pub fn new(target: TargetConfiguration) -> Self {
    FileStreamWriter { target }
  }
}

impl StreamWriter for FileStreamWriter {
  fn write_stream(&self, reader: &mut Read) -> Result<(), FtpError> {
    let destination_file = File::create(&self.target.path)
      .map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::Other, e.to_string())))?;

    let mut file_writer: BufWriter<File> = BufWriter::new(destination_file);
    copy(reader, &mut file_writer).map_err(|e| FtpError::ConnectionError(e))?;
    Ok(())
  }
}

#[derive(Debug)]
pub struct FtpStreamWriter {
  target: TargetConfiguration,
}

impl FtpStreamWriter {
  pub fn new(target: TargetConfiguration) -> Self {
    FtpStreamWriter { target }
  }
}

impl StreamWriter for FtpStreamWriter {
  fn write_stream(&self, stream: &mut Read) -> Result<(), FtpError> {
    let mut ftp_stream = self.target.get_ftp_stream()?;

    let destination_path = Path::new(self.target.path.as_str());
    let destination_directory = destination_path.parent().unwrap_or_else(|| Path::new("/"));
    let filename = destination_path.file_name().unwrap().to_str().unwrap();

    // create destination directories if not exists
    let prefix = self
      .target
      .prefix
      .clone()
      .unwrap_or_else(|| "/".to_string());
    let mut root_dir = PathBuf::from(prefix);
    for folder in destination_directory.iter() {
      if folder == "/" {
        continue;
      }

      root_dir = root_dir.join(folder);
      let pathname = root_dir.to_str().unwrap();
      if ftp_stream.cwd(pathname).is_err() {
        ftp_stream.mkdir(pathname)?;
      }
    }
    ftp_stream.cwd(root_dir.to_str().unwrap())?;

    let mut reader = BufReader::new(stream);
    ftp_stream.put(filename, &mut reader)?;
    let _length = ftp_stream.size(filename)?;

    ftp_stream.quit()?;
    Ok(())
  }
}
