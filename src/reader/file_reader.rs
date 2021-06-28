use crate::{message::StreamData, reader::StreamReader};
use async_std::channel::Sender;
use async_trait::async_trait;
use mcai_worker_sdk::prelude::{warn, McaiChannel};
use std::fs::File;
use std::io::{Error, ErrorKind, Read};

pub struct FileReader {}

#[async_trait]
impl StreamReader for FileReader {
  async fn read_stream(
    &self,
    path: &str,
    sender: Sender<StreamData>,
    channel: Option<McaiChannel>,
  ) -> Result<(), Error> {
    let mut source_file = File::open(path)?;

    if let Ok(metadata) = source_file.metadata() {
      sender.send(StreamData::Size(metadata.len())).await.unwrap();
    }

    loop {
      if let Some(channel) = &channel {
        if channel.lock().unwrap().is_stopped() {
          return Ok(());
        }
      }

      let mut buffer = vec![0; 30 * 1024];
      let readed_size = source_file.read(&mut buffer)?;

      if readed_size == 0 {
        sender.send(StreamData::Eof).await.unwrap();
        return Ok(());
      }

      if let Err(error) = sender
        .send(StreamData::Data(buffer[0..readed_size].to_vec()))
        .await
      {
        if let Some(channel) = &channel {
          if channel.lock().unwrap().is_stopped() && sender.is_closed() {
            warn!(
              "Data channel closed: could not send {} read bytes.",
              readed_size
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
  }
}
