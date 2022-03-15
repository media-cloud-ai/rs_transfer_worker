use crate::{
  reader::{ReaderNotification, StreamReader},
  StreamData,
};
use async_std::channel::Sender;
use async_trait::async_trait;
use std::io::{Cursor, Error, ErrorKind, Read, Seek, SeekFrom};

pub struct CursorReader {
  pub cursor: Cursor<Vec<u8>>,
}

impl From<&str> for CursorReader {
  fn from(content: &str) -> Self {
    Self::from(content.as_bytes())
  }
}

impl From<&[u8]> for CursorReader {
  fn from(content: &[u8]) -> Self {
    CursorReader {
      cursor: Cursor::new(content.to_vec()),
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
    let mut stream = self.cursor.clone();
    let stream_length = stream.seek(SeekFrom::End(0))?;
    stream.seek(SeekFrom::Start(0))?;
    sender.send(StreamData::Size(stream_length)).await.unwrap();
    loop {
      if channel.is_stopped() {
        return Ok(stream_length);
      }

      let mut buffer = vec![0; 30 * 1024];
      let read_size = stream.read(&mut buffer)?;

      if read_size == 0 {
        sender.send(StreamData::Eof).await.unwrap();
        return Ok(stream_length);
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
          return Ok(stream_length);
        }

        return Err(Error::new(
          ErrorKind::Other,
          format!("Could not send read data through channel: {}", error),
        ));
      }
    }
  }
}
