#[macro_use]
extern crate serde_derive;

mod endpoint;
pub mod message;
mod reader;
pub mod secret;
mod writer;

use crate::secret::Secret;
use mcai_worker_sdk::prelude::{JobResult, JsonSchema, McaiChannel, MessageEvent, Result, Version};

pub mod built_info {
  include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Debug, Default)]
pub struct TransferEvent {}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
pub struct TransferWorkerParameters {
  source_path: String,
  source_secret: Option<Secret>,
  destination_path: String,
  destination_secret: Option<Secret>,
}

impl MessageEvent<TransferWorkerParameters> for TransferEvent {
  fn get_name(&self) -> String {
    "Transfer".to_string()
  }

  fn get_short_description(&self) -> String {
    "Move file from any storage".to_string()
  }

  fn get_description(&self) -> String {
    r#"Move any file from a location to an another one else.
It support in input: Local, FTP, S3, HTTP.
It support in output: Local, FTP, S3."#
      .to_string()
  }

  fn get_version(&self) -> Version {
    Version::parse(built_info::PKG_VERSION).expect("unable to locate Package version")
  }

  fn process(
    &self,
    channel: Option<McaiChannel>,
    parameters: TransferWorkerParameters,
    job_result: JobResult,
  ) -> Result<JobResult> {
    message::process(channel, parameters, job_result)
  }
}
