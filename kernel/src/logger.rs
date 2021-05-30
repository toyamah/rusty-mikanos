#![allow(dead_code)]
use core::fmt;

use log::{Level, Metadata, Record};

use log::{LevelFilter, SetLoggerError};

const LOGGER: Logger = Logger;

pub fn init(max_level: LevelFilter) -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(max_level))
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            crate::printk!("{} - {}\n", record.level(), record.args())
        }
    }

    fn flush(&self) {}
}
