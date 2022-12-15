use mcai_worker_sdk::{job::JobResult, publish_job_progression, McaiChannel};
use rs_transfer::{reader::ReaderNotification, writer::WriteJob, Error};

pub struct TransferWriterNotification {
  pub job_result: JobResult,
  pub channel: Option<McaiChannel>,
  pub emit_progressions: bool,
}

impl WriteJob for TransferWriterNotification {
  fn get_str_job_id(&self) -> String {
    self.job_result.get_str_job_id()
  }

  fn progress(&self, progress: u8) -> Result<(), Error> {
    if self.emit_progressions {
      publish_job_progression(self.channel.clone(), self.job_result.get_job_id(), progress)
        .map_err(|_| Error::Other("unable to publish job progression".to_string()))
    } else {
      Ok(())
    }
  }

  fn is_stopped(&self) -> bool {
    self
      .channel
      .as_ref()
      .map(|channel| channel.lock().unwrap().is_stopped())
      .unwrap_or_default()
  }
}

pub struct TransferReaderNotification {
  pub channel: Option<McaiChannel>,
}

impl ReaderNotification for TransferReaderNotification {
  fn is_stopped(&self) -> bool {
    self
      .channel
      .as_ref()
      .map(|channel| channel.lock().unwrap().is_stopped())
      .unwrap_or_default()
  }
}
