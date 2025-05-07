use core::sync::atomic::{AtomicUsize, Ordering};

static MAX_LEVEL_ORDINAL: AtomicUsize = AtomicUsize::new(Level::NONE as usize);

pub fn init() {
    let max_level = get_env_log_level().unwrap_or(Level::NONE);
    MAX_LEVEL_ORDINAL.store(max_level as usize, Ordering::Relaxed);
}

fn get_env_log_level() -> Option<Level> {
    let env_setting = option_env!("LOG")?;

    let level = if env_setting.eq_ignore_ascii_case("none") {
        Level::NONE
    } else if env_setting.eq_ignore_ascii_case("error") {
        Level::ERROR
    } else if env_setting.eq_ignore_ascii_case("warn") {
        Level::WARN
    } else if env_setting.eq_ignore_ascii_case("info") {
        Level::INFO
    } else if env_setting.eq_ignore_ascii_case("debug") {
        Level::DEBUG
    } else if env_setting.eq_ignore_ascii_case("trace") {
        Level::TRACE
    } else {
        Level::NONE
    };
    Some(level)
}

pub fn should_log(level: Level) -> bool {
    if matches!(level, Level::NONE) {
        return false;
    }
    level as usize <= MAX_LEVEL_ORDINAL.load(Ordering::Relaxed)
}

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
        if $crate::console::log::should_log(level) {
            let color_code = level.get_color_code();
            let prefix = level.get_prefix();

            $crate::console::print(format_args!(
                concat!("\x1b[{}m{} ", $fmt, "\x1b[0m\n"),
                color_code, prefix $(, $($arg)+)?)
            )
        };
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
