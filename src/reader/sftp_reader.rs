use std::io::{Error, ErrorKind, Read};

use async_std::sync::Sender;
use async_trait::async_trait;

use crate::endpoint::sftp::SftpEndpoint;
use crate::message::StreamData;
use crate::reader::StreamReader;

use mcai_worker_sdk::{debug, info};

pub struct SftpReader {
  pub hostname: String,
  pub port: Option<u16>,
  pub username: String,
  pub password: Option<String>,
  pub prefix: Option<String>,
}

impl SftpReader {
  fn get_prefix(&self) -> Option<String> {
    self.prefix.clone()
  }
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
    let prefix = self.get_prefix().unwrap_or_else(|| "/".to_string());
    let absolute_path: String = vec![prefix, path.to_string()].join("/");

    let connection = self.get_sftp_stream()?;
    connection.start().map_err(Into::<Error>::into)?;

    let mut sftp_reader = connection
      .read_over_sftp(&absolute_path)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;
    let file_size = sftp_reader
      .get_size()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    debug!("Size of {} remote file: {}", absolute_path, file_size);

    sender.send(StreamData::Size(file_size)).await;

    let mut buffer = vec![];

    info!("Read remote file: {}", absolute_path);
    let read_bytes = sftp_reader.read_to_end(&mut buffer)?;

    debug!("Read {} bytes on {} expected.", read_bytes, file_size);

    sender.send(StreamData::Data(buffer)).await;
    sender.send(StreamData::Eof).await;

    Ok(())
  }
}
