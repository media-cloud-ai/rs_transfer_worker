pub mod message;
#[cfg(feature = "media_probe_and_upload")]
mod probe;
mod transfer_job;

use mcai_worker_sdk::prelude::{
  default_rust_mcai_worker_description, JobResult, JsonSchema, McaiChannel, McaiWorker, Result,
  Version,
};
use rs_transfer::secret::Secret;
use serde::Deserialize;

#[derive(Debug, Default)]
pub struct TransferEvent {}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
pub struct TransferWorkerParameters {
  source_path: String,
  source_secret: Option<Secret>,
  destination_path: String,
  destination_secret: Option<Secret>,
  #[cfg(feature = "media_probe_and_upload")]
  probe_secret: Option<Secret>,
  #[cfg(feature = "media_probe_and_upload")]
  probe_path: Option<String>,
}

default_rust_mcai_worker_description!();

impl McaiWorker<TransferWorkerParameters, RustMcaiWorkerDescription> for TransferEvent {
  fn process(
    &self,
    channel: Option<McaiChannel>,
    parameters: TransferWorkerParameters,
    job_result: JobResult,
  ) -> Result<JobResult> {
    message::process(channel, parameters, job_result)
  }
}
