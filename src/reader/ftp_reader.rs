use crate::endpoint::ftp::FtpEndpoint;
use crate::{message::StreamData, reader::StreamReader};
use async_std::sync::Sender;
use async_trait::async_trait;
use std::io::{Error, ErrorKind};
use std::path::Path;

pub struct FtpReader {
  pub hostname: String,
  pub port: Option<u16>,
  pub secure: Option<bool>,
  pub username: Option<String>,
  pub password: Option<String>,
  pub prefix: Option<String>,
}

impl FtpEndpoint for FtpReader {
  fn get_hostname(&self) -> String {
    self.hostname.clone()
  }

  fn get_port(&self) -> u16 {
    self.port.clone().unwrap_or(21)
  }

  fn is_secure(&self) -> bool {
    self.secure.clone().unwrap_or(false)
  }

  fn get_username(&self) -> Option<String> {
    self.username.clone()
  }

  fn get_password(&self) -> Option<String> {
    self.password.clone()
  }
}

#[async_trait]
impl StreamReader for FtpReader {
  async fn read_stream(&self, path: &str, sender: Sender<StreamData>) -> Result<(), Error> {
    let prefix = self.prefix.clone().unwrap_or_else(|| "/".to_string());
    let absolute_path = prefix + path;

    let path = Path::new(&absolute_path);
    let directory = path.parent().unwrap().to_str().unwrap();
    let filename = path.file_name().unwrap().to_str().unwrap();

    let mut ftp_stream = self
      .get_ftp_stream()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    ftp_stream
      .cwd(&directory)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    if let Some(file_size) = ftp_stream
      .size(&filename)
      .map_err(|e| Error::new(ErrorKind::Other, e))?
    {
      sender.send(StreamData::Size(file_size as u64)).await;
    };

    let mut buffer = vec![];
    let mut result = ftp_stream
      .simple_retr(&filename)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;
    buffer.append(result.get_mut());

    sender.send(StreamData::Data(buffer)).await;
    sender.send(StreamData::Eof).await;
    Ok(())
  }
}

#[test]
pub fn test_ftp_reader_getters() {
  let hostname = "ftp.server.name".to_string();
  let port = None;
  let secure = None;
  let username = Some("user".to_string());
  let password = Some("password".to_string());
  let prefix = None;

  let ftp_reader = FtpReader {
    hostname: hostname.clone(),
    port: port.clone(),
    secure: secure.clone(),
    username: username.clone(),
    password: password.clone(),
    prefix: prefix.clone(),
  };

  assert_eq!(ftp_reader.get_hostname(), hostname);
  assert_eq!(ftp_reader.get_port(), 21);
  assert_eq!(ftp_reader.is_secure(), false);
  assert_eq!(ftp_reader.get_username(), username);
  assert_eq!(ftp_reader.get_password(), password);
}
