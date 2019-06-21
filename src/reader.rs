use crate::target_configuration::TargetConfiguration;

use ftp::FtpError;

use std::fs::File;
use std::io::{BufReader, Cursor, Error, ErrorKind, Read};
use std::path::Path;
use tokio_io::AsyncRead;
use tokio::prelude::*;

pub struct FileReader {
  target: TargetConfiguration,
}

impl FileReader {
  pub fn new(target: TargetConfiguration) -> Self {
    FileReader { target }
  }

  pub fn process_copy<F>(&mut self, streamer: F) -> Result<(), FtpError>
  where
    F: Fn(&mut dyn Read) -> Result<(), FtpError>,
  {
    let source_file = File::open(&self.target.path)
      .map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::Other, e.to_string())))?;

    let mut data_stream = BufReader::new(source_file);
    streamer(&mut data_stream)
  }
}

pub struct FtpReader {
  target: TargetConfiguration,
}

impl FtpReader {
  pub fn new(target: TargetConfiguration) -> Self {
    FtpReader { target }
  }

  pub fn process_copy<F>(&mut self, streamer: F) -> Result<(), FtpError>
  where
    F: Fn(&mut dyn Read) -> Result<(), FtpError>,
  {
    let prefix = self
      .target
      .prefix
      .clone()
      .unwrap_or_else(|| "/".to_string());
    let absolute_path = prefix + &self.target.path;

    let path = Path::new(&absolute_path);
    let directory = path.parent().unwrap().to_str().unwrap();
    let filename = path.file_name().unwrap().to_str().unwrap();

    let mut ftp_stream = self.target.get_ftp_stream()?;
    ftp_stream.cwd(&directory)?;
    ftp_stream.retr(&filename, streamer)
  }
}

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
    let s3_byte_stream = self.target.get_s3_stream()?;
    let async_read = s3_byte_stream.into_async_read();

    struct ByteStream<R>(R);
    impl <R: AsyncRead> Stream for ByteStream<R> {
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
                },
                Ok(Async::NotReady) => Ok(Async::NotReady),
                Err(e) => Err(
                  FtpError::ConnectionError(Error::new(ErrorKind::Other, e.to_string()))
                )
            }
        }
    }
    let byte_stream = ByteStream(async_read);
    let process =
      byte_stream
        .for_each(move |stream| {
          let cursor = Cursor::new(stream);
          let mut reader = BufReader::new(cursor);
          streamer(&mut reader).map_err(|e| FtpError::ConnectionError(Error::new(ErrorKind::Other, e.to_string())))
        })
        .map_err(|e| println!("Error reading byte: {:?}", e));

    tokio::run(process);
    Ok(())
  }
}

#[test]
fn tranfer_s3() {
  use crate::writer::FileStreamWriter;
  use crate::writer::StreamWriter;
  use std::env;
  use rusoto_core::region::Region;

  let access_key = env::var("AWS_ACCESS_KEY").expect("not variable AWS_ACCESS_KEY found in environment");
  let secret_key = env::var("AWS_SECRET_KEY").expect("not variable AWS_SECRET_KEY found in environment");

  let src_conf = TargetConfiguration::new_s3(
    &access_key,
    &secret_key,
    Region::EuCentral1,
    "originserver-prod-usp-replay",
    "672f60b78f8c5/200876616.ismt"
  );

  let mut reader = S3Reader::new(src_conf);
  let dst_conf = TargetConfiguration::new_file("/tmp/200876616.imst");
  let mut writer = FileStreamWriter::new(dst_conf);
  writer.open().unwrap();
  reader.process_copy(move |stream| writer.write_stream(stream)).unwrap();
}
