#[macro_use]
extern crate serde_derive;

mod endpoint;
mod message;
mod reader;
mod writer;

use mcai_worker_sdk::{
  job::JobResult, start_worker, JsonSchema, McaiChannel, MessageError, MessageEvent, Version,
};

pub mod built_info {
  include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Debug, Default)]
struct TransferEvent {}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
enum Secret {
  #[serde(rename = "ftp")]
  Ftp {
    hostname: String,
    port: Option<u16>,
    secure: Option<bool>,
    username: Option<String>,
    password: Option<String>,
    prefix: Option<String>,
  },
  #[serde(rename = "http")]
  Http {
    endpoint: String,
    method: String,
    headers: String,
    body: String,
  },
  #[serde(rename = "local")]
  Local,
  #[serde(rename = "s3")]
  S3 {
    hostname: Option<String>,
    access_key_id: String,
    secret_access_key: String,
    region: Option<String>,
    bucket: String,
  },
}

impl Default for Secret {
  fn default() -> Self {
    Secret::Local
  }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TransferWorkerParameters {
  source_path: String,
  source_secret: Secret,
  destination_path: String,
  destination_secret: Secret,
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
  ) -> Result<JobResult, MessageError> {
    message::process(channel, parameters, job_result)
  }
}

fn main() {
  let message_event = TransferEvent::default();
  start_worker(message_event);
}
