use crate::{message::StreamData, reader::StreamReader};
use async_std::channel::Sender;
use async_trait::async_trait;
use cloud_storage::Client;
use futures::StreamExt;
use mcai_worker_sdk::prelude::warn;
use mcai_worker_sdk::McaiChannel;
use std::io::{Error, ErrorKind};

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
    while let Some(byte) = stream.next().await {
      // On récupère les données qu'on peut
      let buffer = vec![byte.unwrap()];
      let buffer_size = buffer.len();
      if let Err(error) = sender.send(StreamData::Data(buffer)).await {
        if let Some(channel) = &channel {
          if channel.lock().unwrap().is_stopped() && sender.is_closed() {
            warn!(
              "Data channel closed: could not send {} read bytes.",
              buffer_size
            );
            return Ok(());
          }
        }
        return Err(Error::new(
          ErrorKind::Other,
          format!("Could not send read data through channel: {}", error),
        ));
      }
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
