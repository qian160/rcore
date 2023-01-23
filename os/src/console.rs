//! SBI console driver, for text output

use crate::sbi::console_putchar;
use core::fmt::{self, Write};

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
/*  Rust macro uses something like "pattern match", or regular expression
    The pattern $( ... ) means repetition. Furthermore, 
    $( ... )* means match 0 or more times that pattern, while
    $( ... )+ means match 1 or more ...
    $( ... )? means 0 or 1 time ...
    the 2nd "argument" of println "$(, $($arg: tt)+)?" should be clear now.
    note: tt means the type "token tree", which is a very powerful type: 
    either a properly matched pair of brackets: (...), [...], {...}, 
    and everything in between, including nested token trees, or 
    a single token that isn't a bracket, like 114514 and "hello world"

*/
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
