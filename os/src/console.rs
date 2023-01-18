//! SBI console driver, for text output

use crate::sbi::console_putchar;
use core::fmt::{self, Write};

// Unit-like structs, contains no fields
struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

/// print string macro
#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

/// println string macro
#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

pub const END: &'static str = "\x1b[0m";
pub mod color{
    pub const ERROR: &'static str = "\x1b[31m";
    pub const WARN:  &'static str = "\x1b[93m";
    pub const INFO:  &'static str = "\x1b[34m";
    pub const DEBUG: &'static str = "\x1b[32m";
    pub const TRACE: &'static str = "\x1b[95m";
}

/// warn: the output is displayed in yellow
#[macro_export]
macro_rules! error{
    ($fmt: literal $(, $($arg: tt)+)?) => {
        print!("{}[ERROR]", crate::console::color::ERROR);
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
        print!("{}", $crate::console::END);
    };
}

/// warn: the output is displayed in yellow
#[macro_export]
macro_rules! warn{
    ($fmt: literal $(, $($arg: tt)+)?) => {
        print!("{}[WARN]", crate::console::color::WARN);
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
        print!("{}", $crate::console::END);
    };
}

/// info: the output is displayed in blue
#[macro_export]
macro_rules! info{
    ($fmt: literal $(, $($arg: tt)+)?) => {
        print!("{}[INFO]", crate::console::color::INFO);
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
        print!("{}", $crate::console::END);
    };
}

/// debug: the output is displayed in green
#[macro_export]
macro_rules! debug{
    ($fmt: literal $(, $($arg: tt)+)?) => {
        print!("{}[DEBUG]", crate::console::color::DEBUG);
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
        print!("{}", $crate::console::END);
    };
}

/// trace: the output is displayed in grey
#[macro_export]
macro_rules! trace{
    ($fmt: literal $(, $($arg: tt)+)?) => {
        print!("{}[TRACE]", crate::console::color::TRACE);
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
        print!("{}", $crate::console::END);
    };
}