use crate::reader::*;
use crate::target_configuration::*;
use crate::writer::*;
use async_std::{sync, task};
use mcai_worker_sdk::{
  info,
  job::{Job, JobResult, JobStatus},
  McaiChannel, MessageError,
};
use std::io::Error;
use std::thread;

pub enum StreamData {
  Data(Vec<u8>),
  Size(u64),
  Eof,
}

pub fn process(
  channel: Option<McaiChannel>,
  job: &Job,
  job_result: JobResult,
) -> Result<JobResult, MessageError> {
  let destination_target = TargetConfiguration::new(&job, "destination")?;
  let source_target = TargetConfiguration::new(&job, "source")?;

  info!(target: &job.job_id.to_string(), "Source: {:?} --> Destination: {:?}", source_target.get_type(), destination_target.get_type());

  let cloned_job = job.clone();

  let (sender, receiver) = sync::channel(1000);
  let reception_task = thread::spawn(move || {
    task::block_on(async {
      match destination_target.get_type() {
        ConfigurationType::Ftp => {
          let writer = FtpWriter {};
          writer
            .write_stream(destination_target, receiver, channel, &cloned_job)
            .await
        }
        ConfigurationType::LocalFile => {
          let writer = FileWriter {};
          writer
            .write_stream(destination_target, receiver, channel, &cloned_job)
            .await
        }
        ConfigurationType::S3Bucket => {
          let writer = S3Writer {};
          writer
            .write_stream(destination_target, receiver, channel, &cloned_job)
            .await
        }
        ConfigurationType::HttpResource => {
          unimplemented!();
        }
      }
    })
  });

  start_reader(source_target, sender).map_err(|e| {
    let result = job_result
      .clone()
      .with_status(JobStatus::Error)
      .with_message(&e.to_string());
    MessageError::ProcessingError(result)
  })?;

  let reception_result = reception_task.join().map_err(|_e| {
    let result = job_result
      .clone()
      .with_status(JobStatus::Error)
      .with_message("error during source data reception");
    MessageError::ProcessingError(result)
  })?;

  reception_result.map_err(|e| {
    let result = job_result
      .clone()
      .with_status(JobStatus::Error)
      .with_message(&e.to_string());
    MessageError::ProcessingError(result)
  })?;

  Ok(job_result.with_status(JobStatus::Completed))
}

fn start_reader(
  source_target: TargetConfiguration,
  sender: sync::Sender<StreamData>,
) -> Result<(), Error> {
  match source_target.get_type() {
    ConfigurationType::Ftp => {
      let reader = FtpReader {};
      task::block_on(async { reader.read_stream(source_target, sender).await })
    }
    ConfigurationType::HttpResource => {
      let reader = HttpReader {};
      task::block_on(async { reader.read_stream(source_target, sender).await })
    }
    ConfigurationType::LocalFile => {
      let reader = FileReader {};
      task::block_on(async { reader.read_stream(source_target, sender).await })
    }
    ConfigurationType::S3Bucket => {
      let reader = S3Reader {};
      task::block_on(async { reader.read_stream(source_target, sender).await })
    }
  }
}
