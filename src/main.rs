#![feature(async_await)]

extern crate amqp_worker;
extern crate ftp;
extern crate futures;
extern crate futures_util;

#[macro_use]
extern crate log;
extern crate rusoto_core;
extern crate rusoto_credential;
extern crate rusoto_s3;
extern crate simple_logger;
extern crate tokio;
extern crate tokio_io;

use amqp_worker::*;
use log::Level;
use std::env;

mod message;
mod reader;
mod target_configuration;
mod writer;

#[derive(Debug)]
struct FtpEvent {}

impl MessageEvent for FtpEvent {
  fn process(&self, msg: &str) -> Result<u64, MessageError> {
    message::process(msg)
  }
}

static FTP_EVENT: FtpEvent = FtpEvent {};

fn main() {
  if env::var("VERBOSE").is_ok() {
    simple_logger::init_with_level(Level::Debug).unwrap();
  } else {
    simple_logger::init_with_level(Level::Warn).unwrap();
  }

  start_worker(&FTP_EVENT);
}
