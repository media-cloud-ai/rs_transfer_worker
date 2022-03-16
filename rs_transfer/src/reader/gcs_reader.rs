use crate::{message::StreamData, reader::StreamReader};
use async_std::channel::Sender;
use async_trait::async_trait;
use cloud_storage::Client;
use futures_util::StreamExt;
use mcai_worker_sdk::McaiChannel;
use std::io::{Error, ErrorKind};
use mcai_worker_sdk::prelude::warn;

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
    channel: Option<McaiChannel>,
  ) -> Result<(), Error> {
    let client = Client::default();

    let object = client.object().read(&self.bucket, path).await.unwrap();
    let file_size = object.size;

    sender
      .send(StreamData::Size(file_size as u64))
      .await
      .unwrap();

    let stream = client
      .object()
      .download_streamed(&self.bucket, path)
      .await
      .unwrap();

    let mut chunks = stream.chunks(BUFFER_SIZE);

    while let Some(chunk) = chunks.next().await {
      let vector = chunk.into_iter().collect::<Result<Vec<u8>, cloud_storage::Error>>().map_err(|error| {
        Error::new(ErrorKind::Other, format!("Cloud Storage Error : {}", error))
      })?;
      send_buffer(&sender, &channel, vector).await?;
    }
    sender.send(StreamData::Eof).await.map_err(|error| {
      Error::new(
        ErrorKind::Other,
        format!("Could not send EOF through channel: {:?}", error),
      )
    })?;
    Ok(())
  }
}

async fn send_buffer(
  sender: &Sender<StreamData>,
  channel: &Option<McaiChannel>,
  buffer: Vec<u8>,
) -> Result<(), Error> {
  if let Err(error) = sender.send(StreamData::Data(buffer.clone())).await {
    if let Some(channel) = &channel {
      if channel.lock().unwrap().is_stopped() && sender.is_closed() {
        warn!("Data channel closed: could not send read bytes.");
        return Ok(());
      }
    }
    return Err(Error::new(
      ErrorKind::Other,
      format!("Could not send read data through channel: {}", error),
    ));
  }
  Ok(())
}
