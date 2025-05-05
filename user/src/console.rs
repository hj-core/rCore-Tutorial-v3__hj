use core::fmt::{self, Write};

const FD_STDOUT: usize = 1;

pub struct Stdout;

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        crate::write(FD_STDOUT, s.as_bytes());
        Ok(())
    }
}

impl Stdout {
    pub fn print(args: fmt::Arguments) {
        Stdout.write_fmt(args).unwrap()
    }
}

#[macro_export]
macro_rules! print {
    ($fmt:literal $(, $($args:tt)+)?) => {
        $crate::console::Stdout::print(format_args!($fmt $(, $($args)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt:literal $(, $($args:tt)+)?) => {
        $crate::console::Stdout::print(format_args!(concat!($fmt, "\n") $(, $($args)+)?));
    }
}
