use crate::reader::*;
use crate::writer::*;
use crate::{Secret, TransferWorkerParameters};
use async_std::{channel, task};
use mcai_worker_sdk::{
  info,
  job::{JobResult, JobStatus},
  McaiChannel, MessageError,
};
use std::io::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime;

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
  let cloned_destination_secret = parameters.destination_secret.unwrap_or_default();
  let cloned_destination_path = parameters.destination_path.clone();
  let cloned_job_result = job_result.clone();

  let source_secret = parameters.source_secret.unwrap_or_default();
  let source_path = parameters.source_path;

  info!(target: &job_result.get_str_job_id(), "Source: {:?} --> Destination: {:?}", source_secret, cloned_destination_secret);

  let runtime = tokio::runtime::Runtime::new().unwrap();
  let runtime = Arc::new(Mutex::new(runtime));
  let s3_writer_runtime = runtime.clone();

  let (sender, receiver) = channel::bounded(1000);
  let reception_task = thread::spawn(move || {
    task::block_on(async {
      start_writer(
        &cloned_destination_path,
        cloned_destination_secret,
        cloned_job_result,
        channel,
        receiver,
        s3_writer_runtime,
      )
      .await
    })
  });

  let sending_task = thread::spawn(move || {
    task::block_on(async {
      start_reader(&source_path, source_secret, sender, runtime.clone()).await
    })
  });

  let sending_result = sending_task.join().map_err(|_e| {
    let result = job_result
      .clone()
      .with_status(JobStatus::Error)
      .with_message("error during source data sending");
    MessageError::ProcessingError(result)
  })?;

  sending_result.map_err(|e| {
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

async fn start_writer(
  cloned_destination_path: &str,
  cloned_destination_secret: Secret,
  cloned_job_result: JobResult,
  channel: Option<McaiChannel>,
  receiver: channel::Receiver<StreamData>,
  runtime: Arc<Mutex<Runtime>>,
) -> Result<(), Error> {
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
        runtime,
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
    Secret::Sftp {
      hostname,
      port,
      username,
      password,
      prefix,
      known_host,
    } => {
      let writer = SftpWriter {
        hostname,
        port,
        username,
        password,
        prefix,
        known_host,
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
}

async fn start_reader(
  source_path: &str,
  source_secret: Secret,
  sender: channel::Sender<StreamData>,
  runtime: Arc<Mutex<Runtime>>,
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
      reader.read_stream(source_path, sender).await
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
      reader.read_stream(source_path, sender).await
    }
    Secret::Local {} => {
      let reader = FileReader {};
      reader.read_stream(source_path, sender).await
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
        runtime,
      };
      reader.read_stream(source_path, sender).await
    }
    Secret::Sftp {
      hostname,
      port,
      username,
      password,
      prefix,
      known_host,
    } => {
      let reader = SftpReader {
        hostname,
        port,
        username,
        password,
        prefix,
        known_host,
      };
      reader.read_stream(source_path, sender).await
    }
  }
}
