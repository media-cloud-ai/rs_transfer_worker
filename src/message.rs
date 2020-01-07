use amqp_worker::job::*;
use amqp_worker::*;

use crate::reader::*;
use crate::target_configuration::*;
use crate::writer::*;

pub fn process(message: &str) -> Result<job::JobResult, MessageError> {
  let job = job::Job::new(message)?;
  debug!(target: &job.job_id.to_string(), "received message: {:?}", job);

  job.check_requirements()?;
  let destination_target = TargetConfiguration::new(&job, "destination")?;
  info!("Destination: {:?}", destination_target.get_type());

  match destination_target.get_type() {
    ConfigurationType::Ftp => {
      let writer = FtpStreamWriter::new(destination_target);

      do_transfer(&job, writer)?;
    }
    ConfigurationType::LocalFile => {
      let mut writer = FileStreamWriter::new(destination_target);
      writer.open().map_err(|e| {
        let result = JobResult::new(job.job_id, JobStatus::Error).with_message(&e.to_string());

        MessageError::ProcessingError(result)
      })?;

      do_transfer(&job, writer)?;
    }
    ConfigurationType::S3Bucket => {
      let writer = S3StreamWriter::new(destination_target);

      do_transfer(&job, writer)?;
    }
    _ => {
      let result = JobResult::new(job.job_id, JobStatus::Error)
        .with_message("Unsupported Writer configuration");

      return Err(MessageError::ProcessingError(result));
    }
  }

  Ok(JobResult::new(job.job_id, JobStatus::Completed))
}

fn do_transfer(job: &job::Job, writer: impl StreamWriter + 'static) -> Result<(), MessageError> {
  let source_target = TargetConfiguration::new(&job, "source")?;
  info!("Source: {:?}", source_target.get_type());

  match source_target.get_type() {
    ConfigurationType::Ftp => {
      let mut ftp_reader = FtpReader::new(source_target);

      ftp_reader
        .process_copy(move |stream| writer.write_stream(stream))
        .map_err(|e| {
          let result = JobResult::new(job.job_id, JobStatus::Error).with_message(&e.to_string());
          MessageError::ProcessingError(result)
        })
    }
    ConfigurationType::S3Bucket => {
      let mut s3_reader = S3Reader::new(source_target, job);

      s3_reader
        .process_copy(move |stream| writer.write_stream(stream))
        .map_err(|e| {
          let result = JobResult::new(job.job_id, JobStatus::Error).with_message(&e.to_string());
          MessageError::ProcessingError(result)
        })
    }
    ConfigurationType::LocalFile => {
      let mut file_reader = FileReader::new(source_target);

      file_reader
        .process_copy(move |stream| writer.write_stream(stream))
        .map_err(|e| {
          let result = JobResult::new(job.job_id, JobStatus::Error).with_message(&e.to_string());
          MessageError::ProcessingError(result)
        })
    }
    ConfigurationType::HttpResource => {
      let mut http_reader = HttpReader::new(source_target, job);

      http_reader
        .process_copy(move |stream| writer.write_stream(stream))
        .map_err(|e| {
          let result = JobResult::new(job.job_id, JobStatus::Error).with_message(&e.to_string());
          MessageError::ProcessingError(result)
        })
    }
  }
}
