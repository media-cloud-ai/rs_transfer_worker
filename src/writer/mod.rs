
mod file_stream_writer;
mod ftp_stream_writer;

use ftp::FtpError;
use std::io::Read;

pub use file_stream_writer::FileStreamWriter;
pub use ftp_stream_writer::FtpStreamWriter;

pub trait StreamWriter: Sized + Send + Sync {
  fn write_stream(&self, read_stream: &mut dyn Read) -> Result<(), FtpError>;
}
