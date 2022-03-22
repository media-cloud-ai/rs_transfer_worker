use crate::{
  error::map_sync_send_error,
  writer::{StreamWriter, WriteJob},
  StreamData,
};
use async_std::channel::Receiver;
use async_trait::async_trait;
use cloud_storage::Client;
use futures_util::StreamExt;
use std::io::{Error, ErrorKind};

#[derive(Clone, Debug)]
pub struct GcsWriter {
  pub bucket: String,
}

#[async_trait]
impl StreamWriter for GcsWriter {
  async fn write_stream(
    &self,
    path: &str,
    receiver: Receiver<StreamData>,
    job_and_notification: &dyn WriteJob,
  ) -> Result<(), Error> {
    let client = Client::default();
    let job_id_str = job_and_notification.get_str_job_id();
    let total_file_size;
    let mut received_bytes = 0;
    let mut prev_percent = 0;

    let first_message = receiver.recv().await.map_err(|error| {
      Error::new(
        ErrorKind::Other,
        format!(
          "Unexpected error on message reception in GCS Writer: {:?}",
          error
        ),
      )
    })?;

    let (progression_sender, progression_receiver) = std::sync::mpsc::sync_channel(100);

    match first_message {
      StreamData::Size(file_size) => {
        total_file_size = file_size;
        client
          .object()
          .create_streamed(
            &self.bucket,
            receiver
              .take_while(move |message| {
                futures::future::ready(
                  !matches!(message, StreamData::Eof) || !matches!(message, StreamData::Stop),
                )
              })
              .map(move |stream_data| match stream_data {
                StreamData::Data(data) => {
                  received_bytes += data.len();
                  progression_sender
                    .send(received_bytes)
                    .map_err(map_sync_send_error)?;
                  Ok(data)
                }
                other => Err(Error::new(
                  ErrorKind::InvalidData,
                  format!("GCS writer received an unexpected message: {:?}", other),
                )),
              }),
            file_size,
            path,
            "application/octet-stream",
          )
          .await
          .map_err(|error| {
            log::error!(
              target: &job_id_str,
              "Could not create {:?} object on remote {:?} bucket: {:?}",
              path,
              self.bucket,
              error
            );
            Error::new(
              ErrorKind::Other,
              format!(
                "Could not create {:?} object on remote {:?} bucket: {:?}",
                path, self.bucket, error
              ),
            )
          })?;
      }
      StreamData::Eof | StreamData::Stop => {
        log::warn!(target: &job_id_str, "Nothing to do in writer.");
        return Ok(());
      }
      other => {
        return Err(Error::new(
          ErrorKind::InvalidData,
          format!(
            "GCS writer received an unexpected {:?} message, Size was expected",
            other
          ),
        ));
      }
    }

    while let Ok(received_bytes) = progression_receiver.recv() {
      let percent = (received_bytes as f32 / total_file_size as f32 * 100.0) as u8;

      if percent > prev_percent {
        prev_percent = percent;
        job_and_notification.progress(percent)?;
      }
    }

    Ok(())
  }
}

#[test]
pub fn test_gcs_writer_invalid_data_message_before_size() {
  use crate::writer::DummyWriteJob;

  let dummy_write_job = DummyWriteJob {};
  let bucket = "test_bucket".to_string();
  let path = "/path/to/destination_file.txt".to_string();
  let message = StreamData::Data(vec![1, 2, 3]);

  let (sender, receiver) = async_std::channel::bounded(100);

  let gcs_writer = GcsWriter { bucket };
  let result = async_std::task::block_on(async {
    sender.send(message).await.unwrap();
    gcs_writer
      .write_stream(&path, receiver, &dummy_write_job)
      .await
  });

  assert!(result.is_err());
  let error = result.as_ref().unwrap_err();
  assert_eq!(ErrorKind::InvalidData, error.kind());
  assert_eq!(
    "GCS writer received an unexpected Data([1, 2, 3]) message, Size was expected",
    error.to_string()
  );
}

#[test]
pub fn test_gcs_writer_eof_message_before_size() {
  use crate::writer::DummyWriteJob;

  let dummy_write_job = DummyWriteJob {};
  let bucket = "test_bucket".to_string();
  let path = "/path/to/destination_file.txt".to_string();
  let message = StreamData::Eof;

  let (sender, receiver) = async_std::channel::bounded(100);

  let gcs_writer = GcsWriter { bucket };
  let result = async_std::task::block_on(async {
    sender.send(message).await.unwrap();
    gcs_writer
      .write_stream(&path, receiver, &dummy_write_job)
      .await
  });

  assert!(result.is_ok());
  assert!(matches!(result.unwrap(), ()));
}

#[test]
pub fn test_gcs_writer_stop_message_before_size() {
  use crate::writer::DummyWriteJob;

  let dummy_write_job = DummyWriteJob {};
  let bucket = "test_bucket".to_string();
  let path = "/path/to/destination_file.txt".to_string();
  let message = StreamData::Stop;

  let (sender, receiver) = async_std::channel::bounded(100);

  let gcs_writer = GcsWriter { bucket };
  let result = async_std::task::block_on(async {
    sender.send(message).await.unwrap();
    gcs_writer
      .write_stream(&path, receiver, &dummy_write_job)
      .await
  });

  assert!(result.is_ok());
  assert!(matches!(result.unwrap(), ()));
}
