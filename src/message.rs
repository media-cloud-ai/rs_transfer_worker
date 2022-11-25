#[cfg(feature = "media_probe_and_upload")]
use crate::probe;
use crate::{
  transfer_job::{TransferReaderNotification, TransferWriterNotification},
  TransferWorkerParameters,
};
use async_std::{channel, task};
use mcai_worker_sdk::prelude::{info, JobResult, JobStatus, McaiChannel, MessageError};
use rs_transfer::{reader::*, secret::Secret, writer::*, StreamData};
use std::{
  io::Error,
  sync::{Arc, Mutex},
  thread,
};
use tokio::runtime::Runtime;

pub fn process(
  channel: Option<McaiChannel>,
  parameters: TransferWorkerParameters,
  job_result: JobResult,
) -> Result<JobResult, MessageError> {
  let cloned_destination_secret = parameters.destination_secret.clone().unwrap_or_default();
  let cloned_destination_path = parameters.destination_path.clone();
  let cloned_job_result = job_result.clone();
  let cloned_reader_channel = channel.clone();
  let cloned_writer_channel = channel.clone();
  let cloned_source_path = parameters.source_path.clone();
  let emit_progressions = parameters.emit_progressions;
  #[cfg(feature = "media_probe_and_upload")]
  let source_secret = parameters.source_secret.clone().unwrap_or_default();
  #[cfg(not(feature = "media_probe_and_upload"))]
  let source_secret = parameters.source_secret.unwrap_or_default();

  info!(target: &job_result.get_str_job_id(), "Source: {:?} --> Destination: {:?}", source_secret, cloned_destination_secret);

  let runtime = tokio::runtime::Runtime::new().unwrap();
  let runtime = Arc::new(Mutex::new(runtime));
  let s3_writer_runtime = runtime.clone();

  let (sender, receiver) = channel::bounded(1000);
  let reception_task = thread::spawn(move || {
    task::block_on(async {
      let job_and_notification = TransferWriterNotification {
        job_result: cloned_job_result,
        channel: cloned_writer_channel,
        emit_progressions,
      };

      start_writer(
        &cloned_destination_path,
        &cloned_destination_secret,
        &job_and_notification,
        receiver,
        s3_writer_runtime,
      )
      .await
    })
  });

  let sending_task = thread::spawn(move || {
    task::block_on(async {
      let channel = TransferReaderNotification {
        channel: cloned_reader_channel,
      };
      start_reader(
        &cloned_source_path,
        &source_secret,
        sender,
        runtime.clone(),
        &channel,
      )
      .await
    })
  });

  let sending_result = sending_task.join().map_err(|_e| {
    let result = job_result
      .clone()
      .with_status(JobStatus::Error)
      .with_message("error during source data sending");
    MessageError::ProcessingError(result)
  })?;

  #[cfg(feature = "media_probe_and_upload")]
  let file_size = sending_result.map_err(|e| {
    let result = job_result
      .clone()
      .with_status(JobStatus::Error)
      .with_message(&e.to_string());
    MessageError::ProcessingError(result)
  })?;

  #[cfg(not(feature = "media_probe_and_upload"))]
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

  if let Some(channel) = &channel {
    if channel.lock().unwrap().is_stopped() {
      return Ok(job_result.with_status(JobStatus::Stopped));
    }
  }

  #[cfg(feature = "media_probe_and_upload")]
  {
    if parameters.probe_secret.is_none() {
      let result = job_result
        .with_status(JobStatus::Error)
        .with_message("Missing probe_secret");
      return Err(MessageError::ProcessingError(result));
    }

    let local_file_name = match (
      parameters.source_secret.unwrap_or_default(),
      parameters.destination_secret.unwrap_or_default(),
    ) {
      (Secret::Local(_), _) => Some((
        parameters.source_path.clone(),
        parameters.destination_path.clone(),
      )),
      (_, Secret::Local(_)) => Some((
        parameters.destination_path.clone(),
        parameters.source_path.clone(),
      )),
      (_, _) => None,
    };

    if let Some((local_file_name, name_of_the_file)) = local_file_name {
      let probe_metadata =
        probe::media_probe(&local_file_name, &name_of_the_file, file_size).unwrap();
      probe::upload_metadata(
        job_result.clone(),
        &probe_metadata,
        parameters.probe_secret.unwrap(),
        parameters.probe_path,
      )
      .unwrap();
    };
  }

  Ok(job_result.with_status(JobStatus::Completed))
}

pub async fn start_writer(
  destination_path: &str,
  destination_secret: &Secret,
  job_and_notification: &dyn WriteJob,
  receiver: channel::Receiver<StreamData>,
  runtime: Arc<Mutex<Runtime>>,
) -> Result<(), Error> {
  match destination_secret {
    Secret::Ftp(ftp_secret) => {
      let writer = FtpWriter::from(ftp_secret);
      writer
        .write_stream(destination_path, receiver, job_and_notification)
        .await
    }
    Secret::Gcs(gcs_secret) => {
      std::env::set_var("SERVICE_ACCOUNT_JSON", gcs_secret.credential.to_json()?);

      let writer = GcsWriter::from(gcs_secret);
      writer
        .write_stream(destination_path, receiver, job_and_notification)
        .await
    }
    Secret::Local(file_secret) => {
      let writer = FileWriter::from(file_secret);
      writer
        .write_stream(destination_path, receiver, job_and_notification)
        .await
    }
    Secret::Cursor(_) | Secret::Http(_) => {
      unimplemented!();
    }
    Secret::S3(s3_secret) => {
      let writer = S3Writer::from((s3_secret, runtime));
      writer
        .write_stream(destination_path, receiver, job_and_notification)
        .await
    }
    Secret::Sftp(sftp_secret) => {
      let writer = SftpWriter::from(sftp_secret);
      writer
        .write_stream(destination_path, receiver, job_and_notification)
        .await
    }
  }
}

pub(crate) async fn start_reader(
  source_path: &str,
  source_secret: &Secret,
  sender: channel::Sender<StreamData>,
  runtime: Arc<Mutex<Runtime>>,
  channel: &dyn ReaderNotification,
) -> Result<u64, Error> {
  match source_secret {
    Secret::Ftp(ftp_secret) => {
      let reader = FtpReader::from(ftp_secret);
      reader.read_stream(source_path, sender, channel).await
    }
    Secret::Gcs(gcs_secret) => {
      std::env::set_var("SERVICE_ACCOUNT_JSON", gcs_secret.credential.to_json()?);

      let reader = GcsReader::from(gcs_secret);
      reader.read_stream(source_path, sender, channel).await
    }
    Secret::Http(http_secret) => {
      let reader = HttpReader::from(http_secret);
      reader.read_stream(source_path, sender, channel).await
    }
    Secret::Local(file_secret) => {
      let reader = FileReader::from(file_secret);
      reader.read_stream(source_path, sender, channel).await
    }
    Secret::S3(s3_secret) => {
      let reader = S3Reader::from((s3_secret, runtime));
      reader.read_stream(source_path, sender, channel).await
    }
    Secret::Sftp(sftp_secret) => {
      let reader = SftpReader::from(sftp_secret);
      reader.read_stream(source_path, sender, channel).await
    }
    Secret::Cursor(cursor_secret) => {
      let reader = CursorReader::from(cursor_secret);
      reader.read_stream(source_path, sender, channel).await
    }
  }
}
