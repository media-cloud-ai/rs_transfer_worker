use crate::endpoint::sftp::SftpEndpoint;
use crate::message::StreamData;
use crate::reader::StreamReader;
use async_std::channel::Sender;
use async_trait::async_trait;
use mcai_worker_sdk::{debug, info};
use ssh_transfer::KnownHost;
use std::convert::TryFrom;
use std::io::{Error, ErrorKind, Read};

pub struct SftpReader {
  pub hostname: String,
  pub port: Option<u16>,
  pub username: String,
  pub password: Option<String>,
  pub prefix: Option<String>,
  pub known_host: Option<String>,
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

    let mut connection = self.get_sftp_stream()?;

    if let Some(known_host) = &self.known_host {
      let known_host = KnownHost::try_from(known_host.as_str()).map_err(Into::<Error>::into)?;
      connection
        .add_known_host(&known_host)
        .map_err(Into::<Error>::into)?;
    }

    connection.start().map_err(Into::<Error>::into)?;

    let mut sftp_reader = connection
      .read_over_sftp(&absolute_path)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;
    let file_size = sftp_reader
      .get_size()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    debug!("Size of {} remote file: {}", absolute_path, file_size);

    sender.send(StreamData::Size(file_size)).await.unwrap();

    info!("Start reading remote file {}...", absolute_path);

    let buffer_size = if let Ok(buffer_size) = std::env::var("SFTP_READER_BUFFER_SIZE") {
      buffer_size.parse::<u32>().map_err(|_| {
        Error::new(
          ErrorKind::Other,
          "Unable to parse SFTP_READER_BUFFER_SIZE variable",
        )
      })? as usize
    } else {
      1024 * 1024
    };

    let mut total_read_bytes = 0;

    loop {
      let mut buffer = vec![0; buffer_size];
      let read_size = sftp_reader.read(&mut buffer)?;

      if read_size == 0 {
        sender.send(StreamData::Eof).await.unwrap();
        debug!("Read {} bytes on {} expected.", total_read_bytes, file_size);
        return Ok(());
      }

      total_read_bytes += read_size;

      sender
        .send(StreamData::Data(buffer[0..read_size].to_vec()))
        .await
        .unwrap();
    }
  }
}
