use crate::{
  StreamData,
  reader::{StreamReader, ReaderNotification}
};
use async_std::channel::Sender;
use async_trait::async_trait;
use std::{
  fs::File,
  io::{Error, ErrorKind, Read},
};

pub struct FileReader {}

#[async_trait]
impl StreamReader for FileReader {
  async fn read_stream(
    &self,
    path: &str,
    sender: Sender<StreamData>,
    channel: &dyn ReaderNotification,
  ) -> Result<(), Error> {
    let mut source_file = File::open(path)?;

    if let Ok(metadata) = source_file.metadata() {
      sender.send(StreamData::Size(metadata.len())).await.unwrap();
    }

    loop {
      if channel.is_stopped() {
        return Ok(());
      }

      let mut buffer = vec![0; 30 * 1024];
      let read_size = source_file.read(&mut buffer)?;

      if read_size == 0 {
        sender.send(StreamData::Eof).await.unwrap();
        return Ok(());
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
          return Ok(());
        }


        return Err(Error::new(
          ErrorKind::Other,
          format!("Could not send read data through channel: {}", error),
        ));
      }
    }
  }
}
