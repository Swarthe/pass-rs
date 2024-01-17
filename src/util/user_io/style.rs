use owo_colors::AnsiColors;

use supports_color::Stream::{self, Stdout, Stderr};

use std::fmt;

use std::fmt::Display;

/// For a text object that can be styled with ANSI codes.
///
/// The functions only style the object if the terminal and relevant stream
/// support it, otherwise they return an unstyled `StyledMsg`. The
/// [`NO_COLOR`](https://no-color.org/) convention is supported.
///
/// The implementation for `str` performs no heap allocations, and doesn't copy
/// the styled string. It is therefore usable for sensitive strings.
pub trait Style: Display {
    fn as_error(&self) -> StyledMsg<&Self> {
        maybe_colour(self, Stderr, AnsiColors::BrightRed)
    }

    fn as_warning(&self) -> StyledMsg<&Self> {
        maybe_colour(self, Stderr, AnsiColors::Yellow)
    }

    fn as_notice(&self) -> StyledMsg<&Self> {
        maybe_colour(self, Stderr, AnsiColors::Blue)
    }

    fn as_prompt(&self) -> StyledMsg<&Self> {
        maybe_colour(self, Stderr, AnsiColors::Cyan)
    }

    fn as_title(&self) -> StyledMsg<&Self> {
        maybe_colour(self, Stdout, AnsiColors::Green)
    }

    fn as_heading(&self) -> StyledMsg<&Self> {
        maybe_colour(self, Stdout, AnsiColors::BrightMagenta)
    }

    fn as_name(&self) -> StyledMsg<&Self> {
        if supports_color::on_cached(Stdout).is_some() {
            StyledMsg::with_style(self, owo_colors::Style::new().bold())
        } else {
            StyledMsg::new(self)
        }
    }
}

impl Style for str {}

// we cannot use `owo_colors::Styled` or `owo_colors::SupportsColorsDisplay`
// because it can only be created as a double reference (e.g. `Styled<&&str>`),
// which upsets the borrow checker when we try to return it.
pub struct StyledMsg<M> {
    msg: M,
    style: owo_colors::Style
}

impl<M: Display> StyledMsg<M> {
    /// Returns a `StyledMsg` without an associated style.
    fn new(msg: M) -> Self {
        StyledMsg { msg, style: owo_colors::Style::new() }
    }

    fn with_style(msg: M, style: owo_colors::Style) -> Self {
        Self { msg, style }
    }

    fn with_colour(msg: M, colour: AnsiColors) -> Self {
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

/// If styling is enabled on `strm`, returns `msg` coloured with `col`.
/// Otherwise, returns an unstyled `StyledMsg`.
fn maybe_colour<M>(msg: &M, strm: Stream, col: AnsiColors) -> StyledMsg<&M>
    where
        M: Display + ?Sized
{
    if supports_color::on_cached(strm).is_some() {
        StyledMsg::with_colour(msg, col)
    } else {
        StyledMsg::new(msg)
    }
}
