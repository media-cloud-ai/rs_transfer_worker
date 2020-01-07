use crate::target_configuration::TargetConfiguration;
use crate::writer::StreamWriter;

use ftp::FtpError;
use std::io::{Error, ErrorKind, Read};

#[derive(Clone, Debug)]
pub struct S3StreamWriter {
  target: TargetConfiguration,
}

impl S3StreamWriter {
  pub fn new(target: TargetConfiguration) -> Self {
    S3StreamWriter { target }
  }
}

impl StreamWriter for S3StreamWriter {
  fn write_stream<T: Sized + Read>(&self, mut read_stream: T) -> Result<(), FtpError> {
    let upload_identifier = self.target.start_multi_part_s3_upload()?;
    let mut part_number = 0;
    let mut complete_parts = vec![];

    let part_size = 10 * 1024 * 1024; // limited to 10000 parts = 100Go
    let mut part_buffer: Vec<u8> = Vec::with_capacity(part_size);

    loop {
      let mut tmp_buffer = vec![0; 4096];
      match read_stream.read(&mut tmp_buffer) {
        Ok(0) => {
          complete_parts.push(self.target.upload_s3_part(
            &upload_identifier,
            part_number,
            part_buffer,
          )?);
          self
            .target
            .complete_s3_upload(upload_identifier, complete_parts)?;
          break;
        }
        Ok(n) => {
          part_buffer.append(&mut tmp_buffer[0..n].to_vec());

          if part_buffer.len() > part_size {
            complete_parts.push(self.target.upload_s3_part(
              &upload_identifier,
              part_number,
              part_buffer.clone(),
            )?);
            part_number += 1;
            part_buffer.clear();
          }
        }
        Err(e) => {
          return Err(FtpError::ConnectionError(Error::new(
            ErrorKind::Other,
            e.to_string(),
          )));
        }
      }
    }

    Ok(())
  }
}
