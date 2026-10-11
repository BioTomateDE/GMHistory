#[macro_export]
macro_rules! cli_panic {
    ($($arg:tt)*) => {
        <$crate::Args as ::clap::CommandFactory>::command().error(::clap::error::ErrorKind::ValueValidation, format!($($arg)*)).exit()
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        eprintln!("\x1B[33mWarning: {}\x1B[0m", format_args!($($arg)*))
    };
}

pub fn print_trace(description: &str, mut error: &dyn core::error::Error) {
    const BRIGHT_RED: &str = "\x1b[91m";
    const RED: &str = "\x1b[31m";
    const RESET: &str = "\x1b[0m";

    eprintln!("{BRIGHT_RED}{description}{RED}");
    eprintln!("> {error}");
    while let Some(source) = error.source() {
        eprintln!("> {source}");
        error = source;
    }
    eprintln!("{RESET}")
}
