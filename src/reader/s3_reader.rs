use crate::target_configuration::TargetConfiguration;

use amqp_worker::job::Job;
use ftp::FtpError;
use std::io::{BufReader, Cursor, Error, ErrorKind, Read};
use std::thread;
use tokio::prelude::*;
use tokio_io::AsyncRead;

pub struct S3Reader {
  job_id: u64,
  target: TargetConfiguration,
}

impl S3Reader {
  pub fn new(target: TargetConfiguration, job: &Job) -> Self {
    S3Reader {
      job_id: job.job_id,
      target,
    }
  }

  pub fn process_copy<F>(&mut self, streamer: F) -> Result<(), FtpError>
  where
    F: (Fn(&mut dyn Read) -> Result<(), FtpError>) + Send + Sync + 'static,
  {
    let s3_byte_stream = self.target.get_s3_download_stream()?;
    let async_read = s3_byte_stream.into_async_read();
    let job_id = self.job_id;

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
        .map_err(move |e| error!(target: &job_id.to_string(), "Error reading byte: {:?}", e));

      tokio::run(process);
      Ok(())
    });

    transfer_thread
      .join()
      .map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::Other, format!("{:?}", e))))?
  }
}
