use crate::reader::*;
use crate::writer::*;
use crate::{Secret, TransferWorkerParameters};
use async_std::{sync, task};
use mcai_worker_sdk::{
  info,
  job::{JobResult, JobStatus},
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
  parameters: TransferWorkerParameters,
  job_result: JobResult,
) -> Result<JobResult, MessageError> {
  let cloned_destination_secret = parameters.destination_secret.clone();
  let cloned_destination_path = parameters.destination_path.clone();
  let cloned_job_result = job_result.clone();

  info!(target: &job_result.get_str_job_id(), "Source: {:?} --> Destination: {:?}", parameters.source_secret, cloned_destination_secret);

  let (sender, receiver) = sync::channel(1000);
  let reception_task = thread::spawn(move || {
    task::block_on(async {
      match cloned_destination_secret {
        Secret::Ftp {
          hostname,
          port,
          secure,
          username,
          password,
          prefix,
        } => {
          let writer = FtpWriter {
            hostname,
            port,
            secure,
            username,
            password,
            prefix,
          };
          writer
            .write_stream(
              &cloned_destination_path,
              receiver,
              channel,
              cloned_job_result,
            )
            .await
        }
        Secret::Http {
          endpoint: _,
          method: _,
          headers: _,
          body: _,
        } => {
          unimplemented!();
        }
        Secret::Local {} => {
          let writer = FileWriter {};
          writer
            .write_stream(
              &cloned_destination_path,
              receiver,
              channel,
              cloned_job_result,
            )
            .await
        }
        Secret::S3 {
          hostname,
          access_key_id,
          secret_access_key,
          region,
          bucket,
        } => {
          let writer = S3Writer {
            hostname,
            access_key_id,
            secret_access_key,
            region,
            bucket,
          };
          writer
            .write_stream(
              &cloned_destination_path,
              receiver,
              channel,
              cloned_job_result,
            )
            .await
        }
      }
    })
  });

  start_reader(&parameters.source_path, parameters.source_secret, sender).map_err(|e| {
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
  source_path: &str,
  source_secret: Secret,
  sender: sync::Sender<StreamData>,
) -> Result<(), Error> {
  match source_secret {
    Secret::Ftp {
      hostname,
      port,
      secure,
      username,
      password,
      prefix,
    } => {
      let reader = FtpReader {
        hostname,
        port,
        secure,
        username,
        password,
        prefix,
      };
      task::block_on(async { reader.read_stream(source_path, sender).await })
    }
    Secret::Http {
      endpoint,
      method,
      headers,
      body,
    } => {
      let reader = HttpReader {
        endpoint,
        method,
        headers,
        body,
      };
      task::block_on(async { reader.read_stream(source_path, sender).await })
    }
    Secret::Local {} => {
      let reader = FileReader {};
      task::block_on(async { reader.read_stream(source_path, sender).await })
    }
    Secret::S3 {
      hostname,
      access_key_id,
      secret_access_key,
      region,
      bucket,
    } => {
      let reader = S3Reader {
        hostname,
        access_key_id,
        secret_access_key,
        region,
        bucket,
      };
      task::block_on(async { reader.read_stream(source_path, sender).await })
    }
  }
}
