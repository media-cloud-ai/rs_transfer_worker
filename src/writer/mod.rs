mod file_stream_writer;
mod ftp_stream_writer;
mod s3_stream_writer;

use ftp::FtpError;
use std::io::Read;

pub use file_stream_writer::FileStreamWriter;
pub use ftp_stream_writer::FtpStreamWriter;
pub use s3_stream_writer::S3StreamWriter;

pub trait StreamWriter: Clone + Sized + Send + Sync {
  fn write_stream<T: Sized + Read>(&self, read_stream: T) -> Result<(), FtpError>;
}

#[test]
fn transfer_s3() {
  use crate::reader::FileReader;
  use crate::target_configuration::TargetConfiguration;
  use rusoto_core::region::Region;
  use std::env;

  let access_key =
    env::var("AWS_ACCESS_KEY").expect("not variable AWS_ACCESS_KEY found in environment");
  let secret_key =
    env::var("AWS_SECRET_KEY").expect("not variable AWS_SECRET_KEY found in environment");
  let bucket = env::var("AWS_BUCKET").expect("not variable AWS_BUCKET found in environment");

  let src_conf = TargetConfiguration::new_file("README.md");
  let mut reader = FileReader::new(src_conf);

  let dst_conf = TargetConfiguration::new_s3(
    &access_key,
    &secret_key,
    Region::Custom {
      name: "us-east-1".to_string(),
      endpoint: "s3.media-io.com".to_string(),
    },
    &bucket,
    "README.md",
  );
  let writer = S3StreamWriter::new(dst_conf);

  reader
    .process_copy(move |stream| writer.write_stream(stream))
    .unwrap();
}
