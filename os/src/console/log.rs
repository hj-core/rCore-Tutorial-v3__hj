#[derive(Clone, Copy)]
pub enum Level {
    NONE,
    ERROR,
    WARN,
    INFO,
    DEBUG,
    TRACE,
}

impl Level {
    pub fn get_color_code(&self) -> usize {
        match self {
            Level::INFO => 34,
            Level::WARN => 93,
            Level::ERROR => 31,
            Level::DEBUG => 32,
            Level::TRACE => 90,
            Level::NONE => 0,
        }
    }

    pub fn get_prefix(&self) -> &str {
        match self {
            Level::INFO => "[ INFO]",
            Level::WARN => "[ WARN]",
            Level::ERROR => "[ERROR]",
            Level::DEBUG => "[DEBUG]",
            Level::TRACE => "[TRACE]",
            Level::NONE => "[ NONE]",
        }
    }
}

#[macro_export]
macro_rules! log {
    ($level:expr, $fmt:literal $(, $($arg:tt)+)?) => {
        let level = $level as $crate::console::log::Level;
        let color_code = level.get_color_code();
        let prefix = level.get_prefix();

        $crate::console::print(format_args!(
            concat!("\x1b[{}m{} ", $fmt, "\x1b[0m\n"),
            color_code, prefix $(, $($arg)+)?)
        );
    };
}

#[macro_export]
macro_rules! error {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        log!($crate::console::log::Level::ERROR, $fmt $(, $($arg)+)?);
    };
}

#[macro_export]
macro_rules! warn {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        log!($crate::console::log::Level::WARN, $fmt $(, $($arg)+)?);
    };
}

#[macro_export]
macro_rules! info {
    ($fmt:literal $(, $($arg:tt)+)?) => {
       log!($crate::console::log::Level::INFO, $fmt $(, $($arg)+)?);
    };
}

#[macro_export]
macro_rules! debug {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        log!($crate::console::log::Level::DEBUG, $fmt $(, $($arg)+)?);
    };
}

#[macro_export]
macro_rules! trace {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        log!($crate::console::log::Level::TRACE, $fmt $(, $($arg)+)?);
    };
}
