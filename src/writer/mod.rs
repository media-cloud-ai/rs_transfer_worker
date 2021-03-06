mod file_writer;
mod ftp_writer;
mod s3_writer;
mod sftp_writer;

use crate::message::StreamData;
use async_std::channel::Receiver;
use mcai_worker_sdk::McaiChannel;
use std::io::Error;

use async_trait::async_trait;
pub use file_writer::FileWriter;
pub use ftp_writer::FtpWriter;
use mcai_worker_sdk::job::JobResult;
pub use s3_writer::S3Writer;
pub use sftp_writer::SftpWriter;

#[async_trait]
pub trait StreamWriter {
  async fn write_stream(
    &self,
    path: &str,
    receiver: Receiver<StreamData>,
    channel: Option<McaiChannel>,
    job_result: JobResult,
  ) -> Result<(), Error>;
}
