extern crate amqp_worker;
extern crate ftp;

#[macro_use]
extern crate log;
extern crate simple_logger;

use amqp_worker::*;
use log::Level;
use std::env;

mod message;

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
