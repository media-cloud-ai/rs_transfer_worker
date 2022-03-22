use crate::{
  endpoint::ftp::FtpEndpoint,
  error::map_async_send_error,
  reader::{ReaderNotification, StreamReader},
  StreamData,
};
use async_std::channel::Sender;
use async_trait::async_trait;
use ftp::FtpError;
use std::{
  io::{Error, ErrorKind},
  path::Path,
};

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
    self.port.unwrap_or(21)
  }

  fn is_secure(&self) -> bool {
    self.secure.unwrap_or(false)
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
  async fn read_stream(
    &self,
    path: &str,
    sender: Sender<StreamData>,
    channel: &dyn ReaderNotification,
  ) -> Result<u64, Error> {
    let prefix = self.prefix.clone().unwrap_or_else(|| "/".to_string());
    let absolute_path = prefix + path;

    let path = Path::new(&absolute_path);
    let directory = path.parent().unwrap().to_str().unwrap();
    let filename = path.file_name().unwrap().to_str().unwrap();

    let mut ftp_stream = self
      .get_ftp_stream()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    ftp_stream
      .cwd(directory)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    let mut total_file_size = 0;
    if let Some(file_size) = ftp_stream
      .size(filename)
      .map_err(|e| Error::new(ErrorKind::Other, e))?
    {
      total_file_size = file_size;
      sender
        .send(StreamData::Size(file_size as u64))
        .await
        .map_err(map_async_send_error)?;
    }

    let buffer_size = if let Ok(buffer_size) = std::env::var("FTP_READER_BUFFER_SIZE") {
      buffer_size.parse::<u32>().map_err(|_| {
        Error::new(
          ErrorKind::Other,
          "Unable to parse FTP_READER_BUFFER_SIZE variable",
        )
      })? as usize
    } else {
      1024 * 1024
    };

    let total_read_bytes = ftp_stream
      .retr(filename, |reader| -> Result<u64, FtpError> {
        let mut total_read_bytes: u64 = 0;
        loop {
          if channel.is_stopped() {
            async_std::task::block_on(async {
              sender
                .send(StreamData::Stop)
                .await
                .map_err(|error| FtpError::ConnectionError(map_async_send_error(error)))
            })?;
            return Ok(total_read_bytes);
          }

          let mut buffer = vec![0; buffer_size];
          let read_size = reader
            .read(&mut buffer)
            .map_err(FtpError::ConnectionError)?;

          if read_size == 0 {
            async_std::task::block_on(async {
              sender
                .send(StreamData::Eof)
                .await
                .map_err(|error| FtpError::ConnectionError(map_async_send_error(error)))
            })?;
            log::debug!(
              "Read {} bytes on {} expected.",
              total_read_bytes,
              total_file_size
            );
            return Ok(total_read_bytes);
          }

          total_read_bytes += read_size as u64;

          async_std::task::block_on(async {
            if let Err(error) = sender
              .send(StreamData::Data(buffer[0..read_size].to_vec()))
              .await
            {
              if channel.is_stopped() && sender.is_closed() {
                log::warn!(
                  "Data channel closed: could not send {} read bytes.",
                  read_size
                );
                return Ok(());
              }

              return Err(FtpError::ConnectionError(map_async_send_error(error)));
            }
            Ok(())
          })?;
        }
      })
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    Ok(total_read_bytes)
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
    port,
    secure,
    username: username.clone(),
    password: password.clone(),
    prefix,
  };

  assert_eq!(ftp_reader.get_hostname(), hostname);
  assert_eq!(ftp_reader.get_port(), 21);
  assert!(!ftp_reader.is_secure());
  assert_eq!(ftp_reader.get_username(), username);
  assert_eq!(ftp_reader.get_password(), password);
}
