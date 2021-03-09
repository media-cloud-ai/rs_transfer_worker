use crate::endpoint::ftp::FtpEndpoint;
use crate::message::StreamData;
use crate::writer::StreamWriter;
use async_std::sync::Receiver;
use async_trait::async_trait;
use ftp::FtpStream;
use mcai_worker_sdk::job::JobResult;
use mcai_worker_sdk::{debug, info, publish_job_progression, McaiChannel};
use std::io::{Error, ErrorKind, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct FtpWriter {
  pub hostname: String,
  pub port: Option<u16>,
  pub secure: Option<bool>,
  pub username: Option<String>,
  pub password: Option<String>,
  pub prefix: Option<String>,
}

impl FtpEndpoint for FtpWriter {
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

fn get_directory(path: &str) -> Vec<String> {
  let destination_path = Path::new(path);
  destination_path
    .parent()
    .unwrap_or_else(|| Path::new("/"))
    .iter()
    .map(|item| item.to_os_string().to_str().unwrap().to_string())
    .collect()
}

fn get_filename(path: &str) -> Result<String, Error> {
  let destination_path = Path::new(path);
  Ok(
    destination_path
      .file_name()
      .ok_or_else(|| Error::new(ErrorKind::Other, "Cannot get destination filename."))?
      .to_str()
      .ok_or_else(|| Error::new(ErrorKind::Other, "Cannot get destination filename as string."))?
      .to_string(),
  )
}

impl FtpWriter {
  async fn upload_file(
    &self,
    ftp_stream: &mut FtpStream,
    path: &str,
    receiver: Receiver<StreamData>,
    channel: Option<McaiChannel>,
    job_result: JobResult,
  ) -> Result<(), Error> {
    let destination_directory = get_directory(path);
    let filename = get_filename(path)?;

    // create destination directories if not exists
    let prefix = self.prefix.clone().unwrap_or_else(|| "/".to_string());
    let mut root_dir = PathBuf::from(prefix);

    for folder in destination_directory.iter() {
      if folder == "/" {
        continue;
      }

      root_dir = root_dir.join(folder);
      let pathname = root_dir.to_str().unwrap();
      if ftp_stream.cwd(pathname).is_err() {
        ftp_stream
          .mkdir(pathname)
          .map_err(|e| Error::new(ErrorKind::Other, e))?;
      }
    }
    ftp_stream
      .cwd(root_dir.to_str().unwrap())
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    let mut file_size = None;
    let mut received_bytes = 0;
    let mut prev_percent = 0;
    let mut min_size = std::usize::MAX;
    let mut max_size = 0;

    debug!(target: &job_result.get_str_job_id(), "Start FTP upload to file: {}, directory: {:?}.", filename, root_dir);
    let mut stream = ftp_stream
      .start_put_file(&filename)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    loop {
      let stream_data = receiver.recv().await;
      match stream_data {
        Ok(StreamData::Size(size)) => file_size = Some(size),
        Ok(StreamData::Eof) => {
          info!(target: &job_result.get_str_job_id(), "packet size: min = {}, max= {}", min_size, max_size);
          stream.flush()?;
          break;
        }
        Ok(StreamData::Data(ref data)) => {
          min_size = std::cmp::min(data.len(), min_size);
          max_size = std::cmp::max(data.len(), max_size);

          received_bytes += data.len();
          if let Some(file_size) = file_size {
            let percent = received_bytes as f32 / file_size as f32 * 100.0;

            if percent as u8 > prev_percent {
              prev_percent = percent as u8;
              publish_job_progression(channel.clone(), job_result.get_job_id(), percent as u8)
                .map_err(|_| Error::new(ErrorKind::Other, "unable to publish job progression"))?;
            }
          }

          stream.write_all(data)?;
        }
        _ => {}
      }
    }
    Ok(())
  }
}

#[async_trait]
impl StreamWriter for FtpWriter {
  async fn write_stream(
    &self,
    path: &str,
    receiver: Receiver<StreamData>,
    channel: Option<McaiChannel>,
    job_result: JobResult,
  ) -> Result<(), Error> {
    let mut ftp_stream = self
      .get_ftp_stream()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    self
      .upload_file(&mut ftp_stream, path, receiver, channel, job_result.clone())
      .await?;

    info!(target: &job_result.get_str_job_id(), "ending FTP data connection");
    ftp_stream
      .finish_put_file()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    info!(target: &job_result.get_str_job_id(), "closing FTP connection");
    ftp_stream
      .quit()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    Ok(())
  }
}

#[test]
pub fn test_ftp_writer_getters() {
  let hostname = "ftp.server.name".to_string();
  let port = None;
  let secure = None;
  let username = Some("user".to_string());
  let password = Some("password".to_string());
  let prefix = None;

  let ftp_writer = FtpWriter {
    hostname: hostname.clone(),
    port: port.clone(),
    secure: secure.clone(),
    username: username.clone(),
    password: password.clone(),
    prefix: prefix.clone(),
  };

  assert_eq!(ftp_writer.get_hostname(), hostname);
  assert_eq!(ftp_writer.get_port(), 21);
  assert_eq!(ftp_writer.is_secure(), false);
  assert_eq!(ftp_writer.get_username(), username);
  assert_eq!(ftp_writer.get_password(), password);
}

#[test]
pub fn test_get_directory() {
  let path = "/path/to/directory/file.ext";
  let directory = get_directory(path);
  assert_eq!(
    directory,
    vec![
      "/".to_string(),
      "path".to_string(),
      "to".to_string(),
      "directory".to_string()
    ]
  );
}

#[test]
pub fn test_get_filename() {
  let path = "/path/to/directory/file.ext";
  let filename = get_filename(path).unwrap();
  assert_eq!(filename, "file.ext");
}
