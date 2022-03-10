use crate::endpoint::ftp::FtpEndpoint;
use crate::writer::{StreamWriter, TransferJobAndWriterNotification};
use crate::StreamData;
use async_std::channel::Receiver;
use async_trait::async_trait;
use ftp::FtpStream;
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
      .ok_or_else(|| {
        Error::new(
          ErrorKind::Other,
          "Cannot get destination filename as string.",
        )
      })?
      .to_string(),
  )
}

impl FtpWriter {
  async fn upload_file(
    &self,
    ftp_stream: &mut FtpStream,
    path: &str,
    receiver: Receiver<StreamData>,
    job_and_notification: &dyn TransferJobAndWriterNotification,
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

    log::debug!(target: &job_and_notification.get_str_id(), "Start FTP upload to file: {}, directory: {:?}.", filename, root_dir);
    let mut stream = ftp_stream
      .start_put_file(&filename)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    loop {
      if job_and_notification.is_stopped() {
        return Ok(());
      }

      let stream_data = receiver.recv().await;
      match stream_data {
        Ok(StreamData::Size(size)) => file_size = Some(size),
        Ok(StreamData::Eof) => {
          log::info!(target: &job_and_notification.get_str_id(), "packet size: min = {}, max= {}", min_size, max_size);
          stream.flush()?;
          break;
        }
        Ok(StreamData::Data(ref data)) => {
          min_size = std::cmp::min(data.len(), min_size);
          max_size = std::cmp::max(data.len(), max_size);

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
    job_and_notification: &dyn TransferJobAndWriterNotification,
  ) -> Result<(), Error> {
    let mut ftp_stream = self
      .get_ftp_stream()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    self
      .upload_file(
        &mut ftp_stream,
        path,
        receiver,
        job_and_notification.clone(),
      )
      .await?;

    log::info!(target: &job_and_notification.get_str_id(), "ending FTP data connection");
    ftp_stream
      .finish_put_file()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    log::info!(target: &job_and_notification.get_str_id(), "closing FTP connection");
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
    port,
    secure,
    username: username.clone(),
    password: password.clone(),
    prefix,
  };

  assert_eq!(ftp_writer.get_hostname(), hostname);
  assert_eq!(ftp_writer.get_port(), 21);
  assert!(!ftp_writer.is_secure());
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
