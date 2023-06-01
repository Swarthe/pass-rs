use once_cell::sync::Lazy;

use owo_colors::AnsiColors;

use std::{env, fmt};

use std::fmt::Display;

/// Support the [`NO_COLOR`](https://no-color.org/) convention.
static NO_STYLE: Lazy<bool> = Lazy::new(||
    env::var_os("NO_COLOR").is_some()
);

/// returns from the calling function with the passed parameter in a plain
/// (unstyled) `StyledMsg` object, if `NO_STYLE` is true.
macro_rules! return_if_no_style {
    ($msg:expr) => {{
        if *NO_STYLE {
            return StyledMsg {
                msg: $msg,
                style: owo_colors::Style::new()
            }
        }
    }}
}

/// XXX: does nothing if `NO_COLOR` is set
///
/// the implementation for `str` performs no heap allocations, and doesn't copy
/// the styled string. it is therefore usable for sensitive strings.
pub trait Style: Display {
    fn as_error(&self) -> StyledMsg<&Self> {
        return_if_no_style!(self);
        StyledMsg::new(self, AnsiColors::BrightRed)
    }

    fn as_warning(&self) -> StyledMsg<&Self> {
        return_if_no_style!(self);
        StyledMsg::new(self, AnsiColors::Yellow)
    }

    fn as_notice(&self) -> StyledMsg<&Self> {
        return_if_no_style!(self);
        StyledMsg::new(self, AnsiColors::Blue)
    }

    fn as_prompt(&self) -> StyledMsg<&Self> {
        return_if_no_style!(self);
        StyledMsg::new(self, AnsiColors::Cyan)
    }

    fn as_title(&self) -> StyledMsg<&Self> {
        return_if_no_style!(self);
        StyledMsg::new(self, AnsiColors::Green)
    }

    fn as_heading(&self) -> StyledMsg<&Self> {
        return_if_no_style!(self);
        StyledMsg::new(self, AnsiColors::BrightMagenta)
    }

    fn as_name(&self) -> StyledMsg<&Self> {
        return_if_no_style!(self);

        StyledMsg {
            msg: self,
            style: owo_colors::Style::new().bold()
        }
    }
}

impl Style for str {}

// we cannot use `owo_colors::Styled` because it can only be created as a double
// reference (e.g. `Styled<&&str>`), which upsets the borrow checker when we try
// to return it.
pub struct StyledMsg<M> {
    msg: M,
    style: owo_colors::Style
}

impl<M: Display> StyledMsg<M> {
    fn new(msg: M, colour: AnsiColors) -> Self {
        let style = owo_colors::Style::new()
            .bold().color(colour);

        Self { msg, style }
    }
}

impl<M: Display> Display for StyledMsg<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use owo_colors::OwoColorize;

        write!(f, "{}", self.msg.style(self.style))
    }
}
