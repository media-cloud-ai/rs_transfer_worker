use mcai_worker_sdk::job::JobResult;
use mcai_worker_sdk::{publish_job_progression, McaiChannel};
use rs_transfer::reader::ReaderNotification;
use rs_transfer::writer::WriteJob;
use std::io::{Error, ErrorKind};

pub struct TransferWriterNotification {
  pub job_result: JobResult,
  pub channel: Option<McaiChannel>,
}

impl WriteJob for TransferWriterNotification {
  fn get_str_id(&self) -> String {
    self.job_result.get_str_job_id()
  }

  fn progress(&self, progress: u8) -> Result<(), Error> {
    publish_job_progression(self.channel.clone(), self.job_result.get_job_id(), progress)
      .map_err(|_| Error::new(ErrorKind::Other, "unable to publish job progression"))
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
