use crate::StreamData;
use async_std::channel::SendError;
use std::io::{Error, ErrorKind};

pub(crate) fn map_send_error(error: SendError<StreamData>) -> Error {
  Error::new(
    ErrorKind::Other,
    format!(
      "Could not send {:?} message through channel: {:?}",
      error.0, error
    ),
  )
}
