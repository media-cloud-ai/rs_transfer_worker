use crate::{message::StreamData, reader::StreamReader, target_configuration::TargetConfiguration};
use async_std::sync::Sender;
use async_trait::async_trait;
use std::fs::File;
use std::io::{Error, Read};

pub struct FileReader {}

#[async_trait]
impl StreamReader for FileReader {
  async fn read_stream(
    &self,
    target: TargetConfiguration,
    sender: Sender<StreamData>,
  ) -> Result<(), Error> {
    let mut source_file = File::open(&target.path)?;

    if let Ok(metadata) = source_file.metadata() {
      sender.send(StreamData::Size(metadata.len())).await;
    }

    loop {
      let mut buffer = vec![0; 30 * 1024];
      let readed_size = source_file.read(&mut buffer)?;

      if readed_size == 0 {
        sender.send(StreamData::Eof).await;
        return Ok(());
      }

      sender
        .send(StreamData::Data(buffer[0..readed_size].to_vec()))
        .await;
    }
  }
}
