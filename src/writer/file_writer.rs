use crate::{message::StreamData, writer::StreamWriter};
use async_std::channel::Receiver;
use async_trait::async_trait;
use mcai_worker_sdk::prelude::{info, publish_job_progression, JobResult, McaiChannel};
use std::{
  fs::{self, File, OpenOptions},
  io::{BufWriter, Error, ErrorKind, Write},
  path::Path,
};

#[derive(Clone, Debug)]
pub struct FileWriter {}

#[async_trait]
impl StreamWriter for FileWriter {
  async fn write_stream(
    &self,
    path: &str,
    receiver: Receiver<StreamData>,
    channel: Option<McaiChannel>,
    job_result: JobResult,
  ) -> Result<(), Error> {
    let destination_path = Path::new(path);
    let destination_directory = destination_path.parent().unwrap_or_else(|| Path::new("/"));

    fs::create_dir_all(destination_directory)?;
    let _destination_file = File::create(path)?;

    let destination_file = OpenOptions::new()
      .write(true)
      .create(false)
      .append(true)
      .open(path)?;

    let mut file_writer: BufWriter<File> = BufWriter::new(destination_file);

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
          return Ok(());
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
          file_writer.write_all(data)?;
        }
        _ => {}
      }
    }
  }
}
