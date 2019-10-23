mod file_reader;
mod ftp_reader;
mod http_reader;
mod s3_reader;

pub use file_reader::FileReader;
pub use ftp_reader::FtpReader;
pub use http_reader::HttpReader;
pub use s3_reader::S3Reader;

#[test]
fn transfer_ftp() {
  use crate::target_configuration::TargetConfiguration;
  use crate::writer::FileStreamWriter;
  use crate::writer::StreamWriter;
  use std::env;

  let ftp_filename =
    env::var("FTP_FILENAME").expect("not variable FTP_FILENAME found in environment");
  let ftp_hostname =
    env::var("FTP_HOSTNAME").expect("not variable FTP_HOSTNAME found in environment");
  let ftp_username =
    env::var("FTP_USERNAME").expect("not variable FTP_USERNAME found in environment");
  let ftp_password =
    env::var("FTP_PASSWORD").expect("not variable FTP_PASSWORD found in environment");
  let ftp_prefix = env::var("FTP_PREFIX").expect("not variable FTP_PREFIX found in environment");

  let src_conf = TargetConfiguration::new_ftp(
    &ftp_hostname,
    &ftp_username,
    &ftp_password,
    &ftp_prefix,
    &ftp_filename,
  );

  let mut reader = FtpReader::new(src_conf);
  let dst_conf = TargetConfiguration::new_file("/tmp/tranfer_test_ftp.raw");
  let mut writer = FileStreamWriter::new(dst_conf);
  writer.open().unwrap();
  reader
    .process_copy(move |stream| writer.write_stream(stream))
    .unwrap();
}

#[test]
fn transfer_s3() {
  use crate::target_configuration::TargetConfiguration;
  use crate::writer::FileStreamWriter;
  use crate::writer::StreamWriter;
  use amqp_worker::job::Job;
  use rusoto_core::region::Region;
  use std::env;

  let access_key =
    env::var("AWS_ACCESS_KEY").expect("not variable AWS_ACCESS_KEY found in environment");
  let secret_key =
    env::var("AWS_SECRET_KEY").expect("not variable AWS_SECRET_KEY found in environment");
  let bucket = env::var("AWS_BUCKET").expect("not variable AWS_BUCKET found in environment");
  let filename = env::var("AWS_FILENAME").expect("not variable AWS_FILENAME found in environment");

  let src_conf = TargetConfiguration::new_s3(
    &access_key,
    &secret_key,
    Region::Custom {
      name: "us-east-1".to_string(),
      endpoint: "s3.media-io.com".to_string(),
    },
    &bucket,
    &filename,
  );

  let job = Job {
    job_id: 666,
    parameters: vec![],
  };

  let mut reader = S3Reader::new(src_conf, &job);
  let dst_conf = TargetConfiguration::new_file("/tmp/transfer_test_s3.raw");
  let mut writer = FileStreamWriter::new(dst_conf);
  writer.open().unwrap();
  reader
    .process_copy(move |stream| writer.write_stream(stream))
    .unwrap();
}

#[test]
fn transfer_http() {
  use crate::target_configuration::TargetConfiguration;
  use crate::writer::FileStreamWriter;
  use crate::writer::StreamWriter;
  use amqp_worker::job::Job;

  let job = Job {
    job_id: 666,
    parameters: vec![],
  };

  let src_conf = TargetConfiguration::new_http("https://media-io.com");
  let mut reader = HttpReader::new(src_conf, &job);
  let dst_conf = TargetConfiguration::new_file("/tmp/transfer_test_http.raw");
  let mut writer = FileStreamWriter::new(dst_conf);
  writer.open().unwrap();
  reader
    .process_copy(move |stream| writer.write_stream(stream))
    .unwrap();
}
