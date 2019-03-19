use amqp_worker::*;

use crate::reader::*;
use crate::target_configuration::TargetConfiguration;
use crate::writer::*;

pub fn process(message: &str) -> Result<u64, MessageError> {
  let job = job::Job::new(message)?;
  info!("reveived message: {:?}", job);

  job.check_requirements()?;

  let destination_target = TargetConfiguration::new(&job, "destination")?;

  if destination_target.is_ftp_configured() {
    let writer = FtpStreamWriter::new(destination_target);
    do_transfer(&job, writer)?;
  } else {
    let writer = FileStreamWriter::new(destination_target);
    do_transfer(&job, writer)?;
  };

  Ok(job.job_id)
}

fn do_transfer(job: &job::Job, writer: impl StreamWriter) -> Result<(), MessageError> {
  let source_target = TargetConfiguration::new(&job, "source")?;
  if source_target.is_ftp_configured() {
    let mut ftp_reader = FtpReader::new(source_target.clone());

    ftp_reader
      .retr(|stream| writer.write_stream(stream))
      .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))
  } else {
    let mut file_reader = FileReader::new(source_target.clone());

    file_reader
      .retr(|stream| writer.write_stream(stream))
      .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))
  }
}
