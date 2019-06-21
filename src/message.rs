use amqp_worker::*;

use crate::reader::*;
use crate::target_configuration::*;
use crate::writer::*;

pub fn process(message: &str) -> Result<u64, MessageError> {
  let job = job::Job::new(message)?;
  info!("reveived message: {:?}", job);

  job.check_requirements()?;

  let destination_target = TargetConfiguration::new(&job, "destination")?;

  match destination_target.get_type() {
    ConfigurationType::Ftp => {
      let writer = FtpStreamWriter::new(destination_target);
      do_transfer(&job, writer)?;
    }
    ConfigurationType::LocalFile => {
      let mut writer = FileStreamWriter::new(destination_target);
      writer.open()
        .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))?;
      do_transfer(&job, writer)?;
    }
    _ => {
      return Err(MessageError::ProcessingError(job.job_id, "Unsupported Writer configuration".to_string()));
    }
  }

  Ok(job.job_id)
}

fn do_transfer(job: &job::Job, writer: impl StreamWriter + 'static) -> Result<(), MessageError> {
  let source_target = TargetConfiguration::new(&job, "source")?;

  match source_target.get_type() {
    ConfigurationType::Ftp => {
      let mut ftp_reader = FtpReader::new(source_target.clone());

      ftp_reader
        .process_copy(move |stream| writer.write_stream(stream))
        .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))
    },
    ConfigurationType::S3Bucket => {
      let mut s3_reader = S3Reader::new(source_target.clone());

      s3_reader
        .process_copy(move |stream| writer.write_stream(stream))
        .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))
    },
    ConfigurationType::LocalFile => {
    let mut file_reader = FileReader::new(source_target.clone());

    file_reader
      .process_copy(move |stream| writer.write_stream(stream))
      .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))
    },
  }
}
