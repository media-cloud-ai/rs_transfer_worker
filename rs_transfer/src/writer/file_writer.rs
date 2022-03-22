use crate::{
  writer::{StreamWriter, WriteJob},
  StreamData,
};
use async_std::channel::Receiver;
use async_trait::async_trait;
use std::{
  fs::{self, File, OpenOptions},
  io::{BufWriter, Error, Write},
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
    job_and_notification: &dyn WriteJob,
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

    while let Ok(stream_data) = receiver.recv().await {
      match stream_data {
        StreamData::Size(size) => file_size = Some(size),
        StreamData::Stop => break,
        StreamData::Eof => {
          break;
        }
        StreamData::Data(ref data) => {
          received_bytes += data.len();
          if let Some(file_size) = file_size {
            let percent = (received_bytes as f32 / file_size as f32 * 100.0) as u8;

            if percent > prev_percent {
              prev_percent = percent;
              job_and_notification.progress(percent)?;
            }
          }
          file_writer.write_all(data)?;
        }
      }
    }

    Ok(())
  }
}
