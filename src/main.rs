extern crate amqp_worker;
extern crate ftp;
extern crate futures;
extern crate futures_util;

#[macro_use]
extern crate log;
extern crate reqwest;
extern crate rusoto_core;
extern crate rusoto_credential;
extern crate rusoto_s3;
extern crate tokio;
extern crate tokio_io;
extern crate url;

use amqp_worker::*;

mod message;
mod reader;
mod target_configuration;
mod writer;

#[derive(Debug)]
struct FtpEvent {}

impl MessageEvent for FtpEvent {
  fn process(&self, msg: &str) -> Result<job::JobResult, MessageError> {
    message::process(msg)
  }
}

static FTP_EVENT: FtpEvent = FtpEvent {};

fn main() {
  start_worker(&FTP_EVENT);
}
