use crate::{
  error::map_async_send_error,
  reader::{ReaderNotification, StreamReader},
  StreamData,
};
use async_std::channel::Sender;
use async_trait::async_trait;
use std::io::{Cursor, Error, Read};

pub struct CursorReader {
  content: Vec<u8>,
}

impl From<&str> for CursorReader {
  fn from(content: &str) -> Self {
    Self::from(content.as_bytes())
  }
}

impl From<&[u8]> for CursorReader {
  fn from(content: &[u8]) -> Self {
    Self::from(content.to_vec())
  }
}

impl From<Vec<u8>> for CursorReader {
  fn from(content: Vec<u8>) -> Self {
    CursorReader { content }
  }
}

impl From<Option<Vec<u8>>> for CursorReader {
  fn from(content: Option<Vec<u8>>) -> Self {
    CursorReader {
      content: content.unwrap_or_default(),
    }
  }
}

#[async_trait]
impl StreamReader for CursorReader {
  async fn read_stream(
    &self,
    _path: &str,
    sender: Sender<StreamData>,
    channel: &dyn ReaderNotification,
  ) -> Result<u64, Error> {
    let mut total_read_bytes: u64 = 0;
    let mut stream = Cursor::new(self.content.clone());
    let stream_length = self.content.len() as u64;

    sender
      .send(StreamData::Size(stream_length))
      .await
      .map_err(map_async_send_error)?;

    loop {
      if channel.is_stopped() {
        sender
          .send(StreamData::Stop)
          .await
          .map_err(map_async_send_error)?;
        return Ok(total_read_bytes);
      }

      let mut buffer = vec![0; 30 * 1024];
      let read_size = stream.read(&mut buffer)?;
      total_read_bytes += read_size as u64;
      if read_size == 0 {
        sender
          .send(StreamData::Eof)
          .await
          .map_err(map_async_send_error)?;
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

        return Err(map_async_send_error(error));
      }
    }
  }
}
