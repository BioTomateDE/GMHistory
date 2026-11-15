use log::LevelFilter;
use log::Log;
use log::Metadata;
use log::Record;

use crate::progressbar::BAR_MULTI;

struct BarLogger<L: Log>(L);

impl<L: Log> Log for BarLogger<L> {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.0.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        if self.0.enabled(record.metadata()) {
            BAR_MULTI.suspend(|| self.0.log(record));
        }
    }

    fn flush(&self) {
        self.0.flush();
    }
}

fn logger_fmt(buf: &mut env_logger::fmt::Formatter, record: &log::Record) -> std::io::Result<()> {
    use std::io::Write;
    const RESET: &str = "\x1B[0m";
    const UNBOLD: &str = "\x1B[22m";

    let level = record.level();
    let color = match level {
        log::Level::Error => "\x1B[31;1m", // red
        log::Level::Warn => "\x1B[33;1m",  // yellow
        log::Level::Info => "\x1B[34;1m",  // blue
        log::Level::Debug => "\x1B[1m",
        log::Level::Trace => "\x1B[3;1m", // dimmed
    };
    let args = record.args();
    writeln!(buf, "{color}{level}:{UNBOLD} {args}{RESET}")
}

pub fn init_logger() {
    let env = env_logger::Env::default().default_filter_or("info");
    let logger = env_logger::Builder::from_env(env).format(logger_fmt).build();
    let max: LevelFilter = logger.filter();
    log::set_boxed_logger(Box::new(BarLogger(logger))).expect("Logger already set");
    log::set_max_level(max);
}
