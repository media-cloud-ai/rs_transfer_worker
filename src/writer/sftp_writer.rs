use crate::{endpoint::sftp::SftpEndpoint, message::StreamData, writer::StreamWriter};
use async_std::channel::Receiver;
use async_trait::async_trait;
use mcai_worker_sdk::prelude::{debug, info, publish_job_progression, JobResult, McaiChannel};
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
    channel: Option<McaiChannel>,
    job_result: JobResult,
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
    let mut min_size = std::usize::MAX;
    let mut max_size = 0;

    loop {
      if let Some(channel) = &channel {
        if channel.lock().unwrap().is_stopped() {
          return Ok(());
        }
      }

      let stream_data = receiver.recv().await;
      match stream_data {
        Ok(StreamData::Size(size)) => file_size = Some(size),
        Ok(StreamData::Eof) => {
          info!(target: &job_result.get_str_job_id(), "packet size: min = {}, max= {}", min_size, max_size);
          sftp_writer.flush()?;
          break;
        }
        Ok(StreamData::Data(ref data)) => {
          min_size = std::cmp::min(data.len(), min_size);
          max_size = std::cmp::max(data.len(), max_size);
          debug!("Receive {} bytes to write...", data.len());

          received_bytes += data.len();
          if let Some(file_size) = file_size {
            let percent = received_bytes as f32 / file_size as f32 * 100.0;

            if percent as u8 > prev_percent {
              prev_percent = percent as u8;
              publish_job_progression(channel.clone(), job_result.get_job_id(), percent as u8)
                .map_err(|_| Error::new(ErrorKind::Other, "unable to publish job progression"))?;
            }
          }

          sftp_writer.write_all(data)?;
        }
        _ => {}
      }
    }

    Ok(())
  }
}
