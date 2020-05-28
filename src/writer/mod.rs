mod file_writer;
mod ftp_writer;
mod s3_writer;

use crate::message::StreamData;
use crate::target_configuration::TargetConfiguration;
use async_std::sync::Receiver;
use mcai_worker_sdk::{job::Job, McaiChannel};
use std::io::Error;

use async_trait::async_trait;
pub use file_writer::FileWriter;
pub use ftp_writer::FtpWriter;
pub use s3_writer::S3Writer;

#[async_trait]
pub trait StreamWriter {
  async fn write_stream(
    &self,
    target: TargetConfiguration,
    receiver: Receiver<StreamData>,
    channel: Option<McaiChannel>,
    job: &Job,
  ) -> Result<(), Error>;
}
