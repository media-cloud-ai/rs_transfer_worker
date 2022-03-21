use crate::{
  error::map_async_send_error,
  reader::{ReaderNotification, StreamReader},
  StreamData,
};
use async_std::channel::Sender;
use async_trait::async_trait;
use cloud_storage::Client;
use futures_util::StreamExt;
use std::io::{Error, ErrorKind};

const BUFFER_SIZE: usize = 1024 * 1024;

pub struct GcsReader {
  pub bucket: String,
}

#[async_trait]
impl StreamReader for GcsReader {
  async fn read_stream(
    &self,
    path: &str,
    sender: Sender<StreamData>,
    channel: &dyn ReaderNotification,
  ) -> Result<u64, Error> {
    let client = Client::default();

    let object = client
      .object()
      .read(&self.bucket, path)
      .await
      .map_err(|error| {
        Error::new(
          ErrorKind::Other,
          format!(
            "Could not read {:?} object info from remote {:?} bucket: {:?}",
            path, self.bucket, error
          ),
        )
      })?;
    let file_size = object.size;

    let mut total_read_bytes = 0;

    sender
      .send(StreamData::Size(file_size as u64))
      .await
      .map_err(map_async_send_error)?;

    let stream = client
      .object()
      .download_streamed(&self.bucket, path)
      .await
      .map_err(|error| {
        Error::new(
          ErrorKind::Other,
          format!(
            "Could not initialize {:?} object download from remote {:?} bucket: {:?}",
            path, self.bucket, error
          ),
        )
      })?;

    let mut chunks = stream.chunks(BUFFER_SIZE);

    loop {
      if channel.is_stopped() {
        sender
          .send(StreamData::Stop)
          .await
          .map_err(map_async_send_error)?;
        return Ok(total_read_bytes);
      }

      if let Some(chunk) = chunks.next().await {
        let vector = chunk
          .into_iter()
          .collect::<Result<Vec<u8>, cloud_storage::Error>>()
          .map_err(|error| {
            Error::new(ErrorKind::Other, format!("Cloud Storage Error : {}", error))
          })?;
        let chunk_size = vector.len();
        send_buffer(&sender, channel, vector).await?;
        total_read_bytes += chunk_size as u64;
      } else {
        break;
      }
    }

    sender
      .send(StreamData::Eof)
      .await
      .map_err(map_async_send_error)?;
    Ok(total_read_bytes)
  }
}

async fn send_buffer(
  sender: &Sender<StreamData>,
  channel: &dyn ReaderNotification,
  buffer: Vec<u8>,
) -> Result<(), Error> {
  if let Err(error) = sender.send(StreamData::Data(buffer.clone())).await {
    if channel.is_stopped() && sender.is_closed() {
      log::warn!("Data channel closed: could not send read bytes.");
      return Ok(());
    }
    return Err(map_async_send_error(error));
  }
  Ok(())
}
