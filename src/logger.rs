use chrono::prelude::Local;
use colored::Colorize;
use log::{self, Level, LevelFilter, Log, Metadata, Record, SetLoggerError};

/// Logger implements the log::Log crate
pub struct Logger;

static LOGGER: Logger = Logger;

pub fn init(log_level: &str) -> Result<(), SetLoggerError> {
    let lvl_filter = match log_level {
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        _ => LevelFilter::Info,
    };
    log::set_logger(&LOGGER).map(|_| log::set_max_level(lvl_filter))
}

impl Log for Logger {
    /// Determines if a log message would be logged based on the
    /// log_level filed.
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    /// Logs the `Record`.
    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let rd_lvl = record.level();
        let lvl_txt = match rd_lvl {
            Level::Trace => format!("[{}]", rd_lvl.as_str()).blue(),
            Level::Debug => format!("[{}]", rd_lvl.as_str()).purple(),
            Level::Info => format!("[{}]", rd_lvl.as_str()).green(),
            Level::Warn => format!("[{}]", rd_lvl.as_str()).yellow(),
            Level::Error => format!("[{}]", rd_lvl.as_str()).red(),
        };
        println!(
            "{} {} {}",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            lvl_txt.bold(),
            record.args()
        );
    }

    /// Flushes any buffered records.
    fn flush(&self) {}
}
