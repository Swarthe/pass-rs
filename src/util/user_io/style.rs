// TODO: use owo-colors to remove unnecessary allocations (and name leakage)
//  add relevant security comments (no leakage) when done
//  (still check if color is supported tho)
//  use `set_override` if not done by default

use colored::ColoredString;

pub trait Style {
    /// XXX: does nothing if colour is unsupported
    /// - `NO_COLOR` env variable
    /// - unsupported terminal
    fn style(&self, style: MsgStyle) -> ColoredString;

    fn as_error(&self) -> ColoredString {
        self.style(MsgStyle::Error)
    }

    fn as_warning(&self) -> ColoredString {
        self.style(MsgStyle::Warning)
    }

    fn as_notice(&self) -> ColoredString {
        self.style(MsgStyle::Notice)
    }

    fn as_prompt(&self) -> ColoredString {
        self.style(MsgStyle::Prompt)
    }

    fn as_title(&self) -> ColoredString {
        self.style(MsgStyle::Title)
    }

    fn as_heading(&self) -> ColoredString {
        self.style(MsgStyle::Heading)
    }

    fn as_name(&self) -> ColoredString {
        self.style(MsgStyle::Name)
    }
}

#[derive(Clone, Copy)]
pub enum MsgStyle {
    Error,
    Warning,
    Notice,
    Prompt,
    Title,
    Heading,
    Name
}

impl Style for str {
    fn style(&self, style: MsgStyle) -> ColoredString {
        use colored::Colorize;

        match style {
            MsgStyle::Error   => self.bright_red(),
            MsgStyle::Warning => self.yellow(),
            MsgStyle::Notice  => self.blue(),
            MsgStyle::Prompt  => self.cyan(),
            MsgStyle::Title   => self.green(),
            MsgStyle::Heading => self.bright_magenta(),
            MsgStyle::Name    => ColoredString::from(self),
        }.bold()
    }
}
