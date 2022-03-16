use crate::message;
use async_std::{channel, task};
use log::LevelFilter;
use mcai_worker_sdk::job::JobResult;
use mcai_worker_sdk::prelude::JobStatus;
use mcai_worker_sdk::MessageError;
use rs_transfer::{
  reader::{CursorReader, SimpleReader, StreamReader},
  secret::Secret,
  writer::SimpleWriter,
};
use serde::Serialize;
use stainless_ffmpeg::probe::Probe;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Serialize)]
struct FileInfo {
  filename: String,
  #[serde(skip_serializing_if = "is_format_absent")]
  media_probe: Option<Probe>,
  size: u64,
  mime_type: String,
}

fn is_format_absent(media_probe: &Option<Probe>) -> bool {
  media_probe
    .as_ref()
    .map(|probe| probe.format.is_none())
    .unwrap_or_else(|| true)
}

pub fn upload_metadata(
  job_result: JobResult,
  probe_info: &str,
  probe_secret: Secret,
) -> Result<(), MessageError> {
  let (sender, receiver) = channel::bounded(1000);
  let destination_path = format!("job/probe/{}.json", job_result.get_str_job_id());

  let reception_task = thread::spawn(move || {
    task::block_on(async {
      let probe_writer = SimpleWriter {};
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let runtime = Arc::new(Mutex::new(runtime));
      let s3_writer_runtime = runtime.clone();

      message::start_writer(
        &destination_path,
        probe_secret,
        &probe_writer,
        receiver,
        s3_writer_runtime,
      )
      .await
    })
  });

  task::block_on(async {
    let probe_reader = SimpleReader {};
    let cursor_reader = CursorReader::from(probe_info);
    cursor_reader.read_stream("", sender, &probe_reader).await
  })
  .map_err(|_e| {
    let result = job_result
      .clone()
      .with_status(JobStatus::Error)
      .with_message("Error reading cursor with probe info");
    MessageError::ProcessingError(result)
  })?;

  let reception_task = reception_task.join().map_err(|_e| {
    let result = job_result
      .clone()
      .with_status(JobStatus::Error)
      .with_message("Error during probe metadata upload");
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

pub fn media_probe(local_path: &str, filename: &str, filesize: u64) -> Result<String, String> {
  let mime_type = infer::get_from_path(local_path)
    .unwrap_or_default()
    .map(|mime_type| mime_type.to_string())
    .unwrap_or_else(|| "application/octet-stream".to_string());

  let media_probe = if mime_type == "application/octet-stream" {
    let mut probe = Probe::new(local_path);

    probe
      .process(LevelFilter::Off)
      .map_err(|error| format!("Unable to process probe: {}", error))?;

    Some(probe)
  } else {
    None
  };

  let file_info = FileInfo {
    media_probe,
    filename: filename.to_string(),
    size: filesize,
    mime_type,
  };

  serde_json::to_string(&file_info)
    .map_err(|error| format!("Unable to serialize probe result: {:?}", error))
}

#[test]
pub fn test_probe_empty_path() {
  let result = media_probe("", "", 0);
  assert!(result.is_ok());

  let result: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();

  let expected = std::fs::read_to_string("./tests/probe/empty_path.json").unwrap();
  let expected: serde_json::Value = serde_json::from_str(&expected).unwrap();
  assert_eq!(expected, result);
}

#[test]
pub fn test_probe_remote_file() {
  let result = media_probe("https://github.com/avTranscoder/avTranscoder-data/raw/master/video/BigBuckBunny/BigBuckBunny_480p_stereo.avi", "BigBuckBunny_480p_stereo.avi", 237444416);
  assert!(result.is_ok());

  let result: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();

  let expected = std::fs::read_to_string("./tests/probe/result.json").unwrap();
  let expected: serde_json::Value = serde_json::from_str(&expected).unwrap();
  assert_eq!(expected, result);
}
