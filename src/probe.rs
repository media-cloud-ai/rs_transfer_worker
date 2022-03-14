use crate::message;
use async_std::{channel, task};
use log::LevelFilter;
use mcai_worker_sdk::job::JobResult;
use mcai_worker_sdk::prelude::JobStatus;
use mcai_worker_sdk::MessageError;
use rs_transfer::StreamData;
use rs_transfer::{
  secret::Secret,
  writer::{TransferJob, TransferJobAndWriterNotification, WriterNotification},
};
use stainless_ffmpeg::probe::Probe;
use std::io::{Error, ErrorKind, Read};
use std::sync::{Arc, Mutex};
use std::thread;

struct ProbeTransfer {}

impl TransferJobAndWriterNotification for ProbeTransfer {}
impl WriterNotification for ProbeTransfer {}
impl TransferJob for ProbeTransfer {}

pub fn upload_metadata(
  source_path: &str,
  job_result: JobResult,
  probe_info: &str,
  media_probe_secret: Secret,
) -> Result<(), MessageError> {
  let (sender, receiver) = channel::bounded(1000);
  let mut stream = probe_info.as_bytes();
  let destination_path = format!("{}/{}.json", job_result.get_str_job_id(), source_path);

  let probe_transfer = ProbeTransfer {};

  let reception_task = thread::spawn(move || {
    task::block_on(async {
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let runtime = Arc::new(Mutex::new(runtime));
      let s3_writer_runtime = runtime.clone();

      message::start_writer(
        &destination_path,
        media_probe_secret,
        &probe_transfer,
        receiver,
        s3_writer_runtime,
      )
      .await
    })
  });

  task::block_on(async {
    let size = stream.len() as u64;
    sender.send(StreamData::Size(size)).await.unwrap();

    loop {
      let mut buffer = vec![0; 30 * 1024];
      let read_size = stream.read(&mut buffer)?;

      if read_size == 0 {
        sender.send(StreamData::Eof).await.unwrap();
        return Ok(());
      }

      if let Err(error) = sender
        .send(StreamData::Data(buffer[0..read_size].to_vec()))
        .await
      {
        return Err(Error::new(
          ErrorKind::Other,
          format!("Could not send read data through channel: {}", error),
        ));
      }
    }
  })
  .map_err(|_e| {
    let result = job_result
      .clone()
      .with_status(JobStatus::Error)
      .with_message("Error sending fprobe metadata to S3 writer");
    MessageError::ProcessingError(result)
  })?;

  let reception_task = reception_task.join().map_err(|_e| {
    let result = job_result
      .clone()
      .with_status(JobStatus::Error)
      .with_message("Error during fprobe metadata upload");
    MessageError::ProcessingError(result)
  })?;

  reception_task.map_err(|e| {
    let result = job_result
      .clone()
      .with_status(JobStatus::Error)
      .with_message(&e.to_string());
    MessageError::ProcessingError(result)
  })?;

  Ok(())
}

pub fn fprobe(source_path: &str) -> Result<String, String> {
  let mut probe = Probe::new(source_path);
  probe
    .process(LevelFilter::Off)
    .map_err(|error| format!("Unable to process probe: {}", error))?;

  match probe.format {
    Some(_) => serde_json::to_string(&probe)
      .map_err(|error| format!("Unable to serialize probe result: {:?}", error)),
    None => Err(format!("No such file: '{}'", source_path)),
  }
}

#[test]
pub fn test_probe_empty_path() {
  let result = fprobe("");
  assert!(result.is_err());
  assert_eq!("No such file: ''", &result.unwrap_err());
}

#[test]
pub fn test_probe_remote_file() {
  use serde_json::Value;

  let result = fprobe("https://github.com/avTranscoder/avTranscoder-data/raw/master/video/BigBuckBunny/BigBuckBunny_480p_stereo.avi");
  assert!(result.is_ok());

  let result: Value = serde_json::from_str(&result.unwrap()).unwrap();

  let expected = std::fs::read_to_string("./tests/fprobe/result.json").unwrap();
  let expected: Value = serde_json::from_str(&expected).unwrap();
  assert_eq!(expected, result);
}
