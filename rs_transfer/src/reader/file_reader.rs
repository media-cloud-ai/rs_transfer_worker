use crate::{
  error::map_send_error,
  reader::{ReaderNotification, StreamReader},
  StreamData,
};
use async_std::channel::Sender;
use async_trait::async_trait;
use std::{
  fs::File,
  io::{Error, Read},
};

pub struct FileReader {}

#[async_trait]
impl StreamReader for FileReader {
  async fn read_stream(
    &self,
    path: &str,
    sender: Sender<StreamData>,
    channel: &dyn ReaderNotification,
  ) -> Result<u64, Error> {
    let mut source_file = File::open(path)?;

    let metadata = source_file.metadata()?;
    sender
      .send(StreamData::Size(metadata.len()))
      .await
      .map_err(map_send_error)?;

    let mut total_read_bytes: u64 = 0;
    loop {
      if channel.is_stopped() {
        sender
          .send(StreamData::Stop)
          .await
          .map_err(map_send_error)?;
        return Ok(total_read_bytes);
      }

      let mut buffer = vec![0; 30 * 1024];
      let read_size = source_file.read(&mut buffer)?;
      total_read_bytes += read_size as u64;
      if read_size == 0 {
        sender.send(StreamData::Eof).await.map_err(map_send_error)?;
        return Ok(total_read_bytes);
      }

      if let Err(error) = sender
        .send(StreamData::Data(buffer[0..read_size].to_vec()))
        .await
      {
        if channel.is_stopped() && sender.is_closed() {
          log::warn!(
            "Data channel closed: could not send {} read bytes.",
            read_size
          );
          return Ok(total_read_bytes);
        }

        return Err(map_send_error(error));
      }
    }
  }
}
