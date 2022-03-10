pub mod endpoint;
pub mod reader;
pub mod secret;
pub mod writer;

pub enum StreamData {
  Data(Vec<u8>),
  Size(u64),
  Eof,
}
