mod file_writer;
mod ftp_writer;
mod s3_writer;
mod sftp_writer;

use crate::StreamData;
use async_std::channel::Receiver;
use async_trait::async_trait;
pub use file_writer::FileWriter;
pub use ftp_writer::FtpWriter;
pub use s3_writer::S3Writer;
pub use sftp_writer::SftpWriter;
use std::io::Error;

#[async_trait]
pub trait StreamWriter {
  async fn write_stream(
    &self,
    path: &str,
    receiver: Receiver<StreamData>,
    job_and_notification: &dyn WriteJob,
  ) -> Result<(), Error>;
}

pub trait WriteJob: Send + Sync {
  fn get_str_id(&self) -> String {
    "".to_string()
  }
  fn progress(&self, progress: u8) -> Result<(), Error> {
    log::info!("Received progress {}/100", progress);
    Ok(())
  }

  fn is_stopped(&self) -> bool {
    false
  }
}

pub struct SimpleWriter {}

impl WriteJob for SimpleWriter {}
