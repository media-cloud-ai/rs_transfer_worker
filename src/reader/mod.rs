mod file_reader;
mod ftp_reader;
mod http_reader;
mod s3_reader;
mod sftp_reader;

pub use file_reader::FileReader;
pub use ftp_reader::FtpReader;
pub use http_reader::HttpReader;
pub use s3_reader::S3Reader;

use crate::message::StreamData;
use async_std::sync::Sender;
use async_trait::async_trait;
use std::io::Error;

#[async_trait]
pub trait StreamReader {
  async fn read_stream(&self, path: &str, sender: Sender<StreamData>) -> Result<(), Error>;
}
