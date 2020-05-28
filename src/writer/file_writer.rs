use crate::{message::StreamData, target_configuration::TargetConfiguration, writer::StreamWriter};
use async_std::sync::Receiver;
use mcai_worker_sdk::{info, job::Job, publish_job_progression, McaiChannel};
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Error, ErrorKind, Write};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct FileWriter {}

use async_trait::async_trait;

#[async_trait]
impl StreamWriter for FileWriter {
  async fn write_stream(
    &self,
    target: TargetConfiguration,
    receiver: Receiver<StreamData>,
    channel: Option<McaiChannel>,
    job: &Job,
  ) -> Result<(), Error> {
    let destination_path = Path::new(target.path.as_str());
    let destination_directory = destination_path.parent().unwrap_or_else(|| Path::new("/"));

    fs::create_dir_all(destination_directory)?;
    let _destination_file = File::create(&target.path)?;

    let destination_file = OpenOptions::new()
      .write(true)
      .create(false)
      .append(true)
      .open(&target.path)?;

    let mut file_writer: BufWriter<File> = BufWriter::new(destination_file);

    let mut file_size = None;
    let mut received_bytes = 0;
    let mut prev_percent = 0;
    let mut min_size = std::usize::MAX;
    let mut max_size = 0;

    loop {
      let stream_data = receiver.recv().await;
      match stream_data {
        Some(StreamData::Size(size)) => file_size = Some(size),
        Some(StreamData::Eof) => {
          info!(target: &job.job_id.to_string(), "packet size: min = {}, max= {}", min_size, max_size);
          return Ok(());
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
          file_writer.write_all(&data)?;
        }
        _ => {}
      }
    }
  }
}
