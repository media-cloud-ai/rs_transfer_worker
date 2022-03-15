mod cursor_reader;
mod file_reader;
mod ftp_reader;
mod http_reader;
mod s3_reader;
mod sftp_reader;

pub use cursor_reader::CursorReader;
pub use file_reader::FileReader;
pub use ftp_reader::FtpReader;
pub use http_reader::HttpReader;
pub use s3_reader::S3Reader;
pub use sftp_reader::SftpReader;

use crate::StreamData;
use async_std::channel::Sender;
use async_trait::async_trait;
use std::io::Error;

#[async_trait]
pub trait StreamReader {
  async fn read_stream(
    &self,
    path: &str,
    sender: Sender<StreamData>,
    channel: &dyn ReaderNotification,
  ) -> Result<u64, Error>;
}

pub trait ReaderNotification: Sync + Send {
  fn is_stopped(&self) -> bool {
    false
  }
}
