pub mod message;
#[cfg(feature = "media_probe_and_upload")]
mod probe;
mod transfer_job;

use mcai_worker_sdk::prelude::{
  default_rust_mcai_worker_description, info, JobResult, JsonSchema, McaiChannel, McaiWorker,
  McaiWorkerLicense, OpenSourceLicense, Result,
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
  #[serde(default = "default_bool")]
  emit_progressions: bool,
  #[cfg(feature = "media_probe_and_upload")]
  probe_secret: Option<Secret>,
  #[cfg(feature = "media_probe_and_upload")]
  probe_path: Option<String>,
}

fn default_bool() -> bool {
  true
}

default_rust_mcai_worker_description!();

impl McaiWorker<TransferWorkerParameters, RustMcaiWorkerDescription> for TransferEvent {
  fn process(
    &self,
    channel: Option<McaiChannel>,
    parameters: TransferWorkerParameters,
    job_result: JobResult,
  ) -> Result<JobResult> {
    let job_id = &job_result.get_str_job_id();
    info!(target: job_id, "START_JOB");
    let result = message::process(channel, parameters, job_result);
    info!(target: job_id, "END_JOB");
    result
  }
}
