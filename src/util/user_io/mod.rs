use nix::sys::termios;

use nix::sys::termios::{
    Termios,
    LocalFlags
};

use nix::libc::STDIN_FILENO;

use std::{fmt, io};

use std::{
    io::ErrorKind::UnexpectedEof,
    os::unix::io::RawFd
};

pub mod style;

pub use style::{Style, MsgStyle};

// TODO: use new `macro` syntax with proper namespacing when available

/// Prints a formatted message to standard error with a newline, as an error.
#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => {{
        use $crate::util::user_io::Style;

        use ::std::eprintln;
        use ::std::format_args;

        eprintln!(
            "{} {}",
            Style::as_error("error:"),
            format_args!($($arg)*)
        );
    }};
}

/// Prints a formatted message to standard error with a newline, as a warning.
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        use $crate::util::user_io::Style;

        use ::std::eprintln;
        use ::std::format_args;

        eprintln!(
            "{} {}",
            Style::as_warning("warning:"),
            format_args!($($arg)*)
        );
    }};
}

/// Prints a formatted message to standard error with a newline, as a notice.
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        use $crate::util::user_io::Style;

        use ::std::eprintln;
        use ::std::format_args;

        eprintln!(
            "{} {}",
            Style::as_notice("notice:"),
            format_args!($($arg)*)
        );
    }};
}

/// Reads input from the user.
///
/// This macro has two forms. If arguments are passed, it displays them as a
/// formatted prompt to standard error. Otherwise, it displays a simple
/// graphical prompt. In both cases, it will then read a line from standard
/// input.
///
/// Returns the (possible empty) read input stripped of the trailing newline.
///
/// The prompt is repeated until valid input is read (`EOF` is not considered
/// valid).
#[macro_export]
macro_rules! input {
    () => {{
        use $crate::util::user_io::Style;
        use $crate::util::user_io::get_line;

        use ::std::format_args;

        get_line(format_args!(
            "{} ",
            Style::as_prompt(">"),
        ))
    }};

    ($($arg:tt)*) => {{
        use $crate::util::user_io::Style;
        use $crate::util::user_io::get_line;

        use ::std::format_args;

        get_line(format_args!(
            "{} {}",
            Style::as_prompt("::"),
            format_args!($($arg)*)
        ))
    }};
}

/// Displays a formatted prompt to standard error as a yes/no question and
/// interprets user input.
///
/// Returns true or false depending on whether or not the user confirmed
/// the prompt.
///
/// The prompt is repeated until valid input is read (`EOF` is not considered
/// valid).
#[macro_export]
macro_rules! confirm {
    ($($arg:tt)*) => {{
        use $crate::util::user_io::Result;
        use $crate::input;

        use ::std::format_args;

        let result: Result<bool> = loop {
            let input = input!(
                "{} [y/n] ",
                format_args!($($arg)*)
            );

            let line = match input {
                Ok(l) => l,
                Err(e) => break Result::Err(e)
            };

            match line.as_str() {
                "y" => break Result::Ok(true),
                "n" => break Result::Ok(false),
                 _  => continue
            }
        };

        result
    }};
}

pub type Error = io::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// XXX: repeats prompt until user inputs (possible empty) line (refuses EOF)
///   line is returned without trailing newline
pub fn get_line(prompt: fmt::Arguments) -> Result<String> {
    // Repeat until valid input is read.
    Ok(loop {
        eprint!("{prompt}");

        let mut line = match read_line() {
            Ok(l) => l,

            Err(e) => {
                // Simulate a newline if the user closed the stream.
                if e.kind() == UnexpectedEof { eprintln!() };
                return Err(e);
            }
        };

        // Remove the trailing newline entered by the user. `read_line()` always
        // returns a non-empty `String` so this must succeed.
        line.pop();
        break line;
    })
}

/// XXX: reads stdin until EOF
///  useful for reading piped input
pub fn read_stdin() -> Result<String> {
    use std::io::Read;

    let mut stdin = io::stdin().lock();
    let mut result = String::new();

    stdin.read_to_string(&mut result)?;

    Ok(result)
}

/// XXX: hides user input henceforth
///   useful for reading sensitive data, like passwords
pub fn hide_input() -> Result<()> {
    mutate_termios(STDIN_FILENO, |term| {
        term.local_flags.remove(LocalFlags::ECHO);
    })
}

/// XXX: shows user input henceforth
pub fn show_input() -> Result<()> {
    mutate_termios(STDIN_FILENO, |term| {
        term.local_flags.insert(LocalFlags::ECHO);
    })
}

/// Reads a line from standard input.
///
/// The returned [`String`] is never empty, and contains a trailing newline.
/// Receiving EOF is considered an error.
fn read_line() -> Result<String> {
    let mut input = String::new();

    let read_len = io::stdin().read_line(&mut input)?;

    if read_len > 0 {
        Ok(input)
    } else {
        Err(UnexpectedEof.into())
    }
}

/// XXX
/// mutates termios of `fd` (usually stdin/stdout/stderr) with `op`
/// changes are applied immediately (`TCSANOW`)
// We do not use a struct with Termios data because it is a global resource, and
// may be modified through other means which we cannot control
fn mutate_termios<O>(fd: RawFd, op: O) -> io::Result<()>
    where
        O: FnOnce(&mut Termios)
{
    use termios::SetArg;
    use termios::{tcgetattr, tcsetattr};

    let mut term = tcgetattr(fd)?;

    op(&mut term);
    tcsetattr(fd, SetArg::TCSANOW, &term)?;

    Ok(())
}
