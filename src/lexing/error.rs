use color_eyre::owo_colors::OwoColorize;
use serde::de::StdError;
use std::fmt::{Debug, Display, Formatter};
use thiserror::Error;

pub type LexResult<T> = Result<T, LexError>;

#[derive(Error, Debug)]
pub enum LexError {
    #[error("reached end of string")]
    EndOfString,
    #[error("reached end of string while parsing for {}", .0)]
    EndOfStringWhile(&'static str),
    #[error("unknown byte: {:?} (byte {})", char::from(*.0), .0)]
    UnknownByte(u8),
    #[error("unknown symbol")]
    UnknownSymbol,
    #[error(r#"missing brace, should be \left\{{"#)]
    NoBraceLeft,
    #[error(r#"missing brace, should be \right\}}"#)]
    NoBraceRight,
    #[error(r#"missing brace, should be \left(, [, or {{"#)]
    NoLeft,
    #[error(r#"missing brace, should be \right), ], or }}"#)]
    NoRight,
    #[error(r#"bad operatorname, should be \operatorname{{{}}}"#, "name".green())]
    BadOperatorName,
}

#[derive(Debug)]
pub struct ContextError<'a, T> {
    pub string: &'a str,
    pub index: usize,
    pub error: T,
}

impl<T: Display + Debug> StdError for ContextError<'_, T> {}

impl ContextError<'_, LexError> {
    pub fn is_eos(&self) -> bool {
        matches!(self.error, LexError::EndOfString)
    }
}

impl<'a, T> ContextError<'a, T> {
    pub fn map_error<U>(self, func: impl FnOnce(T) -> U) -> ContextError<'a, U> {
        ContextError {
            string: self.string,
            index: self.index,
            error: func(self.error),
        }
    }
}

impl<T: Display> Display for ContextError<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let string = self.string;
        let index = self.index;

        let (string, index, ellipses) = if self.index < 40 {
            (string, index, "")
        } else {
            (&string[self.index - 37..], 40, "...")
        };

        let (string, after_ellipses) = if string.len() < 80 {
            (string, "")
        } else {
            (&string[..80], "...")
        };

        let bar = "|".white();
        writeln!(f, "{}: {}", "error".bright_red().bold(), self.error.bold(),)?;

        write!(f, "{}", "".bright_white())?;
        writeln!(f, "{bar}  ")?;
        write!(f, "{bar}  ")?;
        writeln!(f, "{}{}{}", ellipses, string, after_ellipses,)?;

        write!(f, "{bar}  ")?;
        write!(f, "{}", " ".repeat(index))?;
        writeln!(f, "{}", "^".bright_red().bold())?;

        write!(f, "{bar}  ")?;
        write!(f, "{}", " ".repeat(index))?;
        writeln!(f, "{}", "| here".bright_red().bold())?;

        write!(f, "{bar}  ")?;
        Ok(())
    }
}
