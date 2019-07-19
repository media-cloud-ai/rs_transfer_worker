mod file_stream_writer;
mod ftp_stream_writer;

use ftp::FtpError;
use std::io::Read;

pub use file_stream_writer::FileStreamWriter;
pub use ftp_stream_writer::FtpStreamWriter;

pub trait StreamWriter: Clone + Sized + Send + Sync {
  fn write_stream<T: Sized + Read>(&self, read_stream: T) -> Result<(), FtpError>;
}
