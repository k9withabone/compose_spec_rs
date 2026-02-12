//! contains the error type used by the variable interpolation parser
//!
//! since the calling code expects the error type to be `serde_yaml::Error`,
//! `ParseError` can be converted to that type, using the error information
//! to construct a custom error message

use serde::de::Error;

/// the error type used for variable interpolation
#[derive(Debug)]
pub(super) struct ParseError {
    /// error message displayed to the user
    message: String,
    /// the current output string at the time of failure
    output: String,
    /// the rest of the input at the time of failure
    rest: String,
}

impl ParseError {
    /// construct a new error.
    /// `message` should be a human-readable error message.
    /// `output` is the current state of the parser's output buffer.
    /// `rest` contains the rest of the parser's unparsed input.
    pub(super) const fn new(message: String, output: String, rest: String) -> Self {
        Self {
            message,
            output,
            rest,
        }
    }
}

impl From<ParseError> for serde_yaml::Error {
    fn from(value: ParseError) -> Self {
        let ParseError {
            message,
            output,
            rest,
        } = value;
        let msg = format!("failed ({message}) around {output} ... {rest}");
        Self::custom(msg)
    }
}

/// convenience macro to quickly construct and return a parsing error
/// from a message and the current parser state.
/// the first argument must be the parser, from the second argument onwards
/// it works like format!()
macro_rules! terminate {
    ($slf:ident, $msg:literal $(,)? $($fragments:expr),*) => {{
        let slf: &crate::variable_interpolation::parser::Parser = $slf;
        let message: ::std::string::String = format!($msg, $($fragments),* );
        let output: ::std::string::String = slf.output.to_owned();
        let rest: ::std::string::String = slf.rest.clone().collect();
        return ::std::result::Result::Err(
            crate::variable_interpolation::error::ParseError::new(
                message,
                output,
                rest,
            ),
        );
    }};
}

pub(super) use terminate;
