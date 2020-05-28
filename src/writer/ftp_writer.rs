use crate::message::StreamData;
use crate::target_configuration::TargetConfiguration;
use crate::writer::StreamWriter;
use async_std::sync::Receiver;
use async_trait::async_trait;
use mcai_worker_sdk::{info, job::Job, publish_job_progression, McaiChannel};
use std::io::{Error, ErrorKind, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct FtpWriter {}

fn get_directory(target: &TargetConfiguration) -> Vec<String> {
  let destination_path = Path::new(&target.path);
  destination_path
    .parent()
    .unwrap_or_else(|| Path::new("/"))
    .iter()
    .map(|item| item.to_os_string().to_str().unwrap().to_string())
    .collect()
}

fn get_filename(target: &TargetConfiguration) -> Result<String, Error> {
  let destination_path = Path::new(&target.path);
  Ok(
    destination_path
      .file_name()
      .unwrap()
      .to_str()
      .unwrap()
      .to_string(),
  )
}

#[async_trait]
impl StreamWriter for FtpWriter {
  async fn write_stream(
    &self,
    target: TargetConfiguration,
    receiver: Receiver<StreamData>,
    channel: Option<McaiChannel>,
    job: &Job,
  ) -> Result<(), Error> {
    let mut ftp_stream = target
      .get_ftp_stream()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    let destination_directory = get_directory(&target);
    let filename = get_filename(&target)?;

    // create destination directories if not exists
    let prefix = target.prefix.clone().unwrap_or_else(|| "/".to_string());
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

    let mut stream = ftp_stream
      .start_put_file(&filename)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    loop {
      let stream_data = receiver.recv().await;
      match stream_data {
        Some(StreamData::Size(size)) => file_size = Some(size),
        Some(StreamData::Eof) => {
          info!(target: &job.job_id.to_string(), "packet size: min = {}, max= {}", min_size, max_size);
          stream.flush()?;

          info!(target: &job.job_id.to_string(), "endding FTP data connection");
          ftp_stream
            .finish_put_file()
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

          info!(target: &job.job_id.to_string(), "closing FTP connection");
          ftp_stream
            .quit()
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
          break;
        }
        Some(StreamData::Data(ref data)) => {
          min_size = std::cmp::min(data.len(), min_size);
          max_size = std::cmp::max(data.len(), max_size);

          received_bytes += data.len();
          if let Some(file_size) = file_size {
            let percent = received_bytes as f32 / file_size as f32 * 100.0;

            if percent as u8 > prev_percent {
              prev_percent = percent as u8;
              publish_job_progression(channel.clone(), job, percent as u8)
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
