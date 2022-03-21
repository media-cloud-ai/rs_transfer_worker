use crate::{
  endpoint::sftp::SftpEndpoint,
  writer::{StreamWriter, WriteJob},
  StreamData,
};
use async_std::channel::Receiver;
use async_trait::async_trait;
use ssh_transfer::KnownHost;
use std::{
  convert::TryFrom,
  io::{Error, ErrorKind, Write},
};

#[derive(Clone, Debug)]
pub struct SftpWriter {
  pub hostname: String,
  pub port: Option<u16>,
  pub username: String,
  pub password: Option<String>,
  pub prefix: Option<String>,
  pub known_host: Option<String>,
}

impl SftpEndpoint for SftpWriter {
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
impl StreamWriter for SftpWriter {
  async fn write_stream(
    &self,
    path: &str,
    receiver: Receiver<StreamData>,
    job_and_notification: &dyn WriteJob,
  ) -> Result<(), Error> {
    let prefix = self.prefix.clone().unwrap_or_else(|| "/".to_string());
    let absolute_path: String = vec![prefix, path.to_string()].join("/");

    let mut connection = self.get_sftp_stream()?;

    if let Some(known_host) = &self.known_host {
      let known_host = KnownHost::try_from(known_host.as_str()).map_err(Into::<Error>::into)?;
      connection
        .add_known_host(&known_host)
        .map_err(Into::<Error>::into)?;
    }

    connection.start().map_err(Into::<Error>::into)?;

    let mut sftp_writer = connection
      .write_over_sftp(&absolute_path)
      .map_err(Into::<Error>::into)?;

    let mut file_size = None;
    let mut received_bytes = 0;
    let mut prev_percent = 0;

    while let Ok(stream_data) = receiver.recv().await {
      match stream_data {
        StreamData::Size(size) => file_size = Some(size),
        StreamData::Stop => break,
        StreamData::Eof => {
          sftp_writer.flush()?;
          break;
        }
        StreamData::Data(ref data) => {
          log::debug!(target: &job_and_notification.get_str_job_id(), "Receive {} bytes to write...", data.len());

          received_bytes += data.len();
          if let Some(file_size) = file_size {
            let percent = (received_bytes as f32 / file_size as f32 * 100.0) as u8;

            if percent > prev_percent {
              prev_percent = percent;
              job_and_notification
                .progress(percent)
                .map_err(|_| Error::new(ErrorKind::Other, "unable to publish job progression"))?;
            }
          }

          sftp_writer.write_all(data)?;
        }
      }
    }

    Ok(())
  }
}
