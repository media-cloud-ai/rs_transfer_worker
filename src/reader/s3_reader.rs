use crate::target_configuration::TargetConfiguration;

use ftp::FtpError;

use std::io::{BufReader, Cursor, Error, ErrorKind, Read};
use std::thread;
use tokio::prelude::*;
use tokio_io::AsyncRead;

pub struct S3Reader {
  target: TargetConfiguration,
}

impl S3Reader {
  pub fn new(target: TargetConfiguration) -> Self {
    S3Reader { target }
  }

  pub fn process_copy<F>(&mut self, streamer: F) -> Result<(), FtpError>
  where
    F: (Fn(&mut dyn Read) -> Result<(), FtpError>) + Send + Sync + 'static,
  {
    let s3_byte_stream = self.target.get_s3_download_stream()?;
    let async_read = s3_byte_stream.into_async_read();

    struct ByteStream<R>(R);
    impl<R: AsyncRead> Stream for ByteStream<R> {
      type Item = Vec<u8>;
      type Error = FtpError;
      fn poll(&mut self) -> Result<Async<Option<Vec<u8>>>, FtpError> {
        let mut buf = [0; 4096];
        match self.0.poll_read(&mut buf) {
          Ok(Async::Ready(n)) => {
            if n == 0 {
              Ok(Async::Ready(None))
            } else {
              Ok(Async::Ready(Some(buf[0..n].to_vec())))
            }
          }
          Ok(Async::NotReady) => Ok(Async::NotReady),
          Err(e) => Err(FtpError::ConnectionError(Error::new(
            ErrorKind::Other,
            e.to_string(),
          ))),
        }
      }
    }

    let transfer_thread = thread::spawn(move || {
      let byte_stream = ByteStream(async_read);
      let process = byte_stream
        .for_each(move |stream| {
          let cursor = Cursor::new(stream);
          let mut reader = BufReader::new(cursor);
          streamer(&mut reader)
            .map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::Other, e.to_string())))
        })
        .map_err(|e| println!("Error reading byte: {:?}", e));

      tokio::run(process);
      Ok(())
    });

    transfer_thread
      .join()
      .map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::Other, format!("{:?}", e))))?
  }
}
