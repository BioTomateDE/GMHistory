use core::fmt;

pub type MyResult<T> = Result<T, Error>;

#[derive(Debug, Clone)]
#[must_use]
pub struct Error {
    pub message: String,
    pub causes: Vec<String>,
}

impl Error {
    pub fn new(message: impl IntoString) -> Self {
        Self {
            message: message.into_string(),
            causes: Vec::new(),
        }
    }

    pub fn ctx(mut self, message: impl IntoString) -> Self {
        self.causes.insert(0, self.message);
        self.message = message.into_string();
        self
    }

    pub const fn display(&'_ self, level: Level) -> ErrorDisplay<'_> {
        ErrorDisplay { error: self, level }
    }

    pub fn print(&self, level: Level) {
        bar_println!("{}", self.display(level));
    }

    pub fn exit(&self) -> ! {
        self.print(Level::Error);
        std::process::exit(1);
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self { message, causes } = self;
        write!(f, "{message}")?;
        for cause in causes {
            write!(f, "\n> {cause}")?;
        }
        Ok(())
    }
}

// impl core::error::Error for Error {}

///////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone)]
pub struct ErrorDisplay<'a> {
    error: &'a Error,
    level: Level,
}

impl fmt::Display for ErrorDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}: {}{}{}",
            self.level.name(),
            self.level.ansi_style(),
            self.error,
            Level::RESET_STYLE
        )
    }
}

///////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    const RESET_STYLE: &str = "\x1B[0m";

    pub const fn name(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warn => "Warning",
            Self::Error => "Error",
        }
    }

    const fn ansi_style(self) -> &'static str {
        match self {
            Self::Info => "\x1B[34m", // Blue
            Self::Warn => "\x1B[33m",  // Yellow
            Self::Error => "\x1B[31m", // Red
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
pub trait IntoString {
    fn into_string(self) -> String;
}

impl IntoString for &str {
    fn into_string(self) -> String {
        self.to_owned()
    }
}

impl IntoString for String {
    fn into_string(self) -> String {
        self
    }
}

impl IntoString for std::borrow::Cow<'_, str> {
    fn into_string(self) -> String {
        self.into_owned()
    }
}

impl<S: IntoString, F: FnOnce() -> S> IntoString for F {
    fn into_string(self) -> String {
        self().into_string()
    }
}

pub trait Context<T> {
    fn ctx(self, message: impl IntoString) -> MyResult<T>;
}

impl<T, E: core::error::Error> Context<T> for Result<T, E> {
    fn ctx(self, message: impl IntoString) -> MyResult<T> {
        self.map_err(|e| Error::from(e).ctx(message))
    }
}

impl<T> Context<T> for Option<T> {
    fn ctx(self, message: impl IntoString) -> MyResult<T> {
        self.ok_or_else(|| Error::new(message))
    }
}

impl<E: core::error::Error> From<E> for Error {
    fn from(error: E) -> Self {
        Self {
            message: error.to_string(),
            causes: collect_causes(&error),
        }
    }
}

impl<T> Context<T> for MyResult<T> {
    fn ctx(self, message: impl IntoString) -> Self {
        self.map_err(|e| e.ctx(message))
    }
}

fn collect_causes(mut error: &dyn core::error::Error) -> Vec<String> {
    let mut causes = Vec::new();
    while let Some(source) = error.source() {
        error = source;
        causes.push(upper(source.to_string()));
    }
    causes
}

fn upper(mut string: String) -> String {
    if let Some(slice) = string.get_mut(0..1) {
        slice.make_ascii_uppercase();
    }
    string
}

#[macro_export]
macro_rules! err {
    ($error:ident, $($arg:tt)*) => {
        $crate::error::Error::from($error).ctx(format!($($arg)*))
    };
    ($($arg:tt)*) => {
        $crate::error::Error::new(format!($($arg)*))
    };
}

#[clippy::format_args]
macro_rules! bail {
    ($error:ident, $($arg:tt)*) => {
        return Err($crate::error::Error::from($error).ctx(format!($($arg)*)))
    };
    ($($arg:tt)*) => {
        return Err($crate::error::Error::new(format!($($arg)*)))
    };
}

#[clippy::format_args]
macro_rules! warning {
    ($error:ident, $($arg:tt)*) => {
        $crate::error::Error::from($error).ctx(format!($($arg)*)).print($crate::error::Level::Warn)
    };
    ($($arg:tt)*) => {
        $crate::error::Error::new(format!($($arg)*)).print($crate::error::Level::Warn)
    };
}

#[clippy::format_args]
macro_rules! bar_println {
    ($($arg:tt)*) => {
        $crate::progressbar::bar_println(format_args!( $($arg)* ))
    };
}

pub(crate) use bail;
pub(crate) use bar_println;
pub(crate) use err;
pub(crate) use warning as warn;
