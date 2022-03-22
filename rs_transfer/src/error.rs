use crate::StreamData;
use std::{
  fmt::Debug,
  io::{Error, ErrorKind},
};

pub(crate) fn map_async_send_error(error: async_std::channel::SendError<StreamData>) -> Error {
  Error::new(
    ErrorKind::Other,
    format!(
      "Could not send {:?} message through channel: {:?}",
      error.0, error
    ),
  )
}

pub(crate) fn map_sync_send_error<T: Debug>(error: std::sync::mpsc::SendError<T>) -> Error {
  Error::new(
    ErrorKind::Other,
    format!(
      "Could not send {:?} message through channel: {:?}",
      error.0, error
    ),
  )
}
