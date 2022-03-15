use crate::{message::StreamData, reader::StreamReader};
use async_std::channel::Sender;
use async_trait::async_trait;
use cloud_storage::Client;
use futures::StreamExt;
use mcai_worker_sdk::prelude::warn;
use mcai_worker_sdk::McaiChannel;
use std::io::{Error, ErrorKind};

const BUFFER_SIZE: usize = 1024*1024;

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

    let mut stream = client
      .object()
      .download_streamed(&self.bucket, path)
      .await
      .unwrap();

    let mut buffer = vec![];

    while let Some(byte) = stream.next().await {
      if buffer.len() == BUFFER_SIZE {
        send_buffer(&sender, &channel, buffer.clone()).await?;
        buffer.clear();
      }
      let byte= byte.map_err(|error| {
        Error::new(
          ErrorKind::InvalidData,
          format!("Could not read byte: {:?}", error),
        )
      })?;
      buffer.push(byte);
    }
    send_buffer(&sender, &channel, buffer).await?;
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
        warn!("Data channel closed: could not send {} read bytes.", 42);
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
