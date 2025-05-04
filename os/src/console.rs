use core::fmt::{self, Write};

use crate::sbi;

struct Stdout;

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            sbi::console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt:literal $(, $($args:tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($args)+)?));
    };
}

#[macro_export]
macro_rules! println {
    () => {
        print!("\n")
    };
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}
