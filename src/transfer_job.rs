use std::io::{Error, ErrorKind};
use mcai_worker_sdk::{McaiChannel, publish_job_progression};
use mcai_worker_sdk::job::JobResult;
use rs_transfer::reader::ReaderNotification;
use rs_transfer::writer::{TransferJob, TransferJobAndWriterNotification, WriterNotification};

pub struct TransferJobAndNotification{
    pub job_result: JobResult,
    pub channel: Option<McaiChannel>
}
impl TransferJobAndWriterNotification for TransferJobAndNotification {}

impl TransferJob for TransferJobAndNotification {
    fn get_str_id(&self) -> String {
        self.job_result.get_str_job_id()
    }
}

impl WriterNotification for TransferJobAndNotification {
  fn progress(&self, progress: u8) -> Result<(), Error> {
      publish_job_progression(self.channel.clone(), self.job_result.get_job_id(), progress)
          .map_err(|_| Error::new(ErrorKind::Other, "unable to publish job progression"))
  }

    fn is_stopped(&self) -> bool {
        self.channel.as_ref().map(|channel| channel.lock().unwrap().is_stopped()).unwrap_or_default()
    }
}

pub struct TransferReaderNotification {
    pub channel: Option<McaiChannel>
}

impl ReaderNotification for TransferReaderNotification {
    fn is_stopped(&self) -> bool {
        self.channel.as_ref().map(|channel| channel.lock().unwrap().is_stopped()).unwrap_or_default()
    }
}


