pub mod endpoint;
pub mod reader;
pub mod secret;
pub mod writer;

#[derive(Debug)]
pub enum StreamData {
  Data(Vec<u8>),
  Size(u64),
  Stop,
  Eof,
}
