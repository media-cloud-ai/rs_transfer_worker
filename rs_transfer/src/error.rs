use crate::StreamData;
use std::io::{Error, ErrorKind};

pub(crate) fn map_async_send_error(error: async_std::channel::SendError<StreamData>) -> Error {
  Error::new(
    ErrorKind::Other,
    format!(
      "Could not send {:?} message through channel: {:?}",
      error.0, error
    ),
  )
}
