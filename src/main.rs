#[macro_use]
extern crate log;

use amqp_worker::*;
use lapin_futures::Channel;
use semver::Version;

mod message;
mod reader;
mod target_configuration;
mod writer;

use amqp_worker::worker::{Parameter, ParameterType};

pub mod built_info {
  include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Debug)]
struct TransferEvent {}

impl MessageEvent for TransferEvent {
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
    semver::Version::parse(built_info::PKG_VERSION).expect("unable to locate Package version")
  }

  fn get_parameters(&self) -> Vec<Parameter> {
    vec![
      Parameter {
        identifier: "source_path".to_string(),
        label: "Source path".to_string(),
        kind: vec![ParameterType::String],
        required: true,
      },
      Parameter {
        identifier: "source_hostname".to_string(),
        label: "Source hostname".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "source_username".to_string(),
        label: "Source username".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "source_password".to_string(),
        label: "Source password".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "source_access_key".to_string(),
        label: "Source access key".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "source_secret_key".to_string(),
        label: "Source secret key".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "source_region".to_string(),
        label: "Source region".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "source_prefix".to_string(),
        label: "Source prefix".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "source_port".to_string(),
        label: "Source port".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "source_ssl".to_string(),
        label: "Source ssl".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "destination_path".to_string(),
        label: "Source path".to_string(),
        kind: vec![ParameterType::String],
        required: true,
      },
      Parameter {
        identifier: "destination_hostname".to_string(),
        label: "Destination hostname".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "destination_username".to_string(),
        label: "Destination username".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "destination_password".to_string(),
        label: "Destination password".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "destination_access_key".to_string(),
        label: "Destination access key".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "destination_secret_key".to_string(),
        label: "Destination secret key".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "destination_region".to_string(),
        label: "Destination region".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "destination_prefix".to_string(),
        label: "Destination prefix".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "destination_port".to_string(),
        label: "Destination port".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
      Parameter {
        identifier: "destination_ssl".to_string(),
        label: "Destination ssl".to_string(),
        kind: vec![ParameterType::Credential],
        required: true,
      },
    ]
  }

  fn process(&self, channel: Option<&Channel>, job: &job::Job, job_result: job::JobResult) -> Result<job::JobResult, MessageError> {
    message::process(channel, job, job_result)
  }
}

static TRANSFER_EVENT: TransferEvent = TransferEvent {};

fn main() {
  start_worker(&TRANSFER_EVENT);
}
