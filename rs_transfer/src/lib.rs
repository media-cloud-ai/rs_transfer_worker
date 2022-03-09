pub mod endpoint;
pub mod reader;
pub mod writer;
pub mod secret;

pub enum StreamData {
  Data(Vec<u8>),
  Size(u64),
  Eof,
}