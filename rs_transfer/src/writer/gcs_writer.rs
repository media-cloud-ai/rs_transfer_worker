use crate::{
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
    _job_and_notification: &dyn WriteJob,
  ) -> Result<(), Error> {
    let client = Client::default();

    if let Ok(StreamData::Size(file_size)) = receiver.recv().await {
      client
        .object()
        .create_streamed(
          &self.bucket,
          receiver
            // FIXME check _job_and_notification.is_stopped()
            .take_while(move |message| futures::future::ready(!matches!(message, StreamData::Eof)))
            .map(|stream_data| match stream_data {
              StreamData::Data(data) => Ok(data),
              other => Err(Error::new(
                ErrorKind::Other,
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

    Ok(())
  }
}
