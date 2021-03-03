use std::io::{Error, ErrorKind, Read};
use std::path::Path;

use async_std::sync::Sender;
use async_trait::async_trait;

use crate::endpoint::sftp::SftpEndpoint;
use crate::message::StreamData;
use crate::reader::StreamReader;

pub struct SftpReader {
  pub hostname: String,
  pub port: Option<u16>,
  pub username: String,
  pub password: Option<String>,
}

impl SftpEndpoint for SftpReader {
  fn get_hostname(&self) -> String {
    self.hostname.clone()
  }

  fn get_port(&self) -> u16 {
    self.port.unwrap_or(22)
  }

  fn get_username(&self) -> String {
    self.username.clone()
  }

  fn get_password(&self) -> Option<String> {
    self.password.clone()
  }
}

#[async_trait]
impl StreamReader for SftpReader {
  async fn read_stream(&self, path: &str, sender: Sender<StreamData>) -> Result<(), Error> {
    let path = Path::new(path);

    let sftp = self
      .get_sftp_stream()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    let file_stat = sftp
      .stat(path)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;
    let file_size = file_stat.size.ok_or_else(|| Error::new(
      ErrorKind::Other,
      format!("Cannot retrieve '{:?}' file size.", path),
    ))?;

    sender.send(StreamData::Size(file_size)).await;

    let mut buffer = vec![];
    let mut remote_file = sftp
      .open(path)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;
    let read_bytes = remote_file.read_to_end(&mut buffer)?;

    sender.send(StreamData::Data(buffer)).await;
    sender.send(StreamData::Eof).await;

    Ok(())
  }
}

