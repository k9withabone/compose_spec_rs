//! the parsing and interpolation logic being applied to the yaml strings
use super::error::terminate;
use super::error::ParseError;
use super::VariableResolver;
use std::{iter::Peekable, str::Chars};

/// the result type for the parser
type Result = std::result::Result<(), ParseError>;

#[derive(Debug, Clone)]
/// the struct containing the state of the parser during the parsing process
pub(super) struct Parser<'s> {
    /// the current rest of the input source
    rest: Peekable<Chars<'s>>,
    /// the current state of the output
    output: String,
    /// the source of the variable values
    env: &'s VariableResolver,
}

#[derive(Debug, Clone)]
/// variables can be modified to replace an empty or unset variable with a default value
/// or to replace a generic failure with a better error message.
/// encodes all the ways a braced variable can be modified:
/// - "NAME-DEFAULT": use DEFAULT if variable NAME is not set
/// - "NAME:-DEFAULT": use DEFAULT if variable NAME is not set or empty
/// - "NAME?MESSAGE": fail with MESSAGE if variable NAME is not set
/// - "NAME:?MESSAGE": fail with MESSAGE if variabele NAME is not set or empty
enum Modifier {
    /// if the condition matches, replace the failure with the error message
    ErrorMessage(ModifierCondition, String),
    /// if the condition matches, replace the variable name with the default
    DefaultValue(ModifierCondition, String),
}

#[derive(Debug, Clone)]
/// encodes when the modifier value should be applied
enum ModifierCondition {
    /// apply when the variable is not set or when it is empty
    WhenUnsetOrEmpty,
    /// apply only when the variable is not set
    WhenUnset,
}

impl<'s> Parser<'s> {
    /// parse the given `String` and return a new `String`, replacing  any encountered
    /// variables with the value returned by `vars`
    pub(super) fn start(vars: &'s VariableResolver, source: &'s str) -> serde_yaml::Result<String> {
        let mut parser = Self {
            rest: source.chars().peekable(),
            output: String::with_capacity(source.len()),
            env: vars,
        };
        parser
            .parse_string()
            .map_err(Into::<serde_yaml::Error>::into)?;
        Ok(parser.output)
    }

    /// entry point into the parsing logic. after this returns Ok, self.output
    /// contains the interpolated input string.
    fn parse_string(&mut self) -> Result {
        loop {
            if self.rest.peek().is_none() {
                // nothing to parse anymore, we're done
                break;
            }

            self.parse_variable_start()?;
            self.parse_literal();
        }

        Ok(())
    }

    /// parse either the start of a variable name ($) or an escaped dollar sign ($$)
    fn parse_variable_start(&mut self) -> Result {
        // check that the iterator points to a $ and consume it
        match self.rest.peek() {
            Some('$') => {
                let _ = self.rest.next();
            }
            _ => return Ok(()),
        }

        // check if it's escaped. if so, consume & push $ to output and return.
        // if not, continue with a braced or non-braced variable.
        match self.rest.peek() {
            Some('$') => {
                let _ = self.rest.next();
                self.output.push('$');
                Ok(())
            }
            Some('{') => self.parse_braced_variable_start(),
            _ => self.parse_variable_name(),
        }
    }

    /// parse a character that's not part of a variable name and not a `$`
    fn parse_literal(&mut self) {
        match self.rest.peek() {
            Some('$') | None => (),
            Some(character) => {
                self.output.push(*character);
                let _ = self.rest.next();
            }
        }
    }

    /// parse a braced variable: `{NAME}`
    fn parse_braced_variable_start(&mut self) -> Result {
        // check that the iterator points to a `{`
        match self.rest.next() {
            Some('{') => {}
            Some(character) => terminate!(self, "expected {{, got {}", character),
            None => terminate!(self, "input ended unexpectedly"),
        }
        self.parse_braced_variable_name()
    }

    /// parse the name of an unbraced variable and replace it with its value
    fn parse_variable_name(&mut self) -> Result {
        let mut name = String::new();
        match self.rest.next() {
            Some(character @ ('_' | 'a'..='z' | 'A'..='Z')) => name.push(character),
            Some(character) => {
                terminate!(
                    self,
                    "expected one of $, {{, _, a-z, A-Z, got {}",
                    character
                )
            }
            None => terminate!(self, "input ended unexpectedly"),
        }

        // consume & push until we know the variable name ended.
        while let Some(character @ ('a'..='z' | 'A'..='Z' | '0'..='9' | '_')) = self.rest.peek() {
            name.push(*character);
            let _ = self.rest.next();
        }

        if let Some(value) = self.env.get(&name) {
            self.output.push_str(value);
        } else {
            terminate!(self, "unbound variable: {}", name)
        }

        Ok(())
    }

    /// parse a braced variable name and push the associated value into `output`
    fn parse_braced_variable_name(&mut self) -> Result {
        let mut name = String::new();
        self.consume_whitespace();
        // the next character should be the start of a variable name.
        match self.rest.next() {
            Some(character @ ('_' | 'a'..='z' | 'A'..='Z')) => name.push(character),
            Some(character) => terminate!(
                self,
                "expected _, a-z, A-Z or whitespace, got {}",
                character
            ),
            None => terminate!(self, "input ended unexpectedly"),
        }

        // consume & push until we know the variable name ended.
        let mut modifier: Option<Modifier> = None;
        loop {
            match self.rest.peek() {
                Some('}') => {
                    let _ = self.rest.next();
                    break;
                }
                Some(':' | '?' | '-') => {
                    modifier.replace(self.parse_modifier()?);
                    break;
                }
                Some(whitespace_char) if whitespace_char.is_whitespace() => {
                    self.rest.next();
                    // Whitespace marks end of variable name
                    // Proceed forward until some variable-name terminator is found
                    self.consume_whitespace();
                    match self.rest.peek() {
                        Some('}') => {
                            self.rest.next();
                            break;
                        }
                        Some(':' | '?' | '-') => {
                            modifier.replace(self.parse_modifier()?);
                            break;
                        }
                        Some(&c) => {
                            terminate!(self, "expected one of }}:?- , got {}", c);
                        }
                        None => terminate!(self, "input ended unexpectedly"),
                    }
                }
                Some(c) => {
                    name.push(*c);
                    let _ = self.rest.next();
                }
                None => terminate!(self, "input ended unexpectedly"),
            }
        }

        let value: &str = if let Some(value) = self.env.get(&name) {
            if value.is_empty() {
                use Modifier::{DefaultValue, ErrorMessage};
                use ModifierCondition::{WhenUnset, WhenUnsetOrEmpty};

                match modifier {
                    Some(DefaultValue(WhenUnsetOrEmpty, ref default)) => default,
                    Some(ErrorMessage(WhenUnsetOrEmpty, err_msg)) => {
                        terminate!(self, "empty variable: {} ({})", name, err_msg)
                    }
                    Some(DefaultValue(WhenUnset, _) | ErrorMessage(WhenUnset, _)) | None => "",
                }
            } else {
                value
            }
        } else {
            use Modifier::{DefaultValue, ErrorMessage};
            use ModifierCondition::{WhenUnset, WhenUnsetOrEmpty};
            match modifier {
                Some(DefaultValue(WhenUnset | WhenUnsetOrEmpty, ref default)) => default,
                Some(ErrorMessage(WhenUnset | WhenUnsetOrEmpty, err_msg)) => {
                    terminate!(self, "unbound variable: {} ({})", name, err_msg)
                }
                None => {
                    terminate!(self, "unbound variable: {}", name)
                }
            }
        };

        self.output.push_str(value);
        Ok(())
    }

    /// consume characters from source until the first non-whitespace character is encountered
    fn consume_whitespace(&mut self) {
        while self
            .rest
            .next_if(|c: &char| char::is_whitespace(*c))
            .is_some()
        {}
    }

    /// check for a modifier or default value. these can contain nested variables.
    fn parse_modifier(&mut self) -> std::result::Result<Modifier, ParseError> {
        use Modifier::{DefaultValue, ErrorMessage};

        let cond = match self.rest.peek() {
            Some(':') => {
                let _ = self.rest.next();
                ModifierCondition::WhenUnsetOrEmpty
            }
            // error cases (EOF, missing ? or -) are handled below.
            _ => ModifierCondition::WhenUnset,
        };

        let mut mod_val = String::new();
        let modifier: Modifier = match self.rest.next() {
            Some('-') => {
                self.parse_modifier_value(&mut mod_val)?;
                DefaultValue(cond, mod_val)
            }
            Some('?') => {
                self.parse_modifier_value(&mut mod_val)?;
                ErrorMessage(cond, mod_val)
            }
            Some(character) => terminate!(self, "expected one of ? or -, got {}", character),
            None => terminate!(self, "input ended unexpectedly"),
        };

        Ok(modifier)
    }

    /// parse the contents of the error message or default value of a braced variable
    fn parse_modifier_value(&mut self, output: &mut String) -> Result {
        loop {
            match self.rest.peek() {
                Some('}') => {
                    let _ = self.rest.next();
                    break;
                }
                Some('$') => self.parse_variable_start()?,
                Some(character) => {
                    output.push(*character);
                    let _ = self.rest.next();
                }
                None => terminate!(self, "input ended unexpectedly"),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // false positive: variable placeholders look a lot like rust formatting args
    #![allow(clippy::literal_string_with_formatting_args)]
    use crate::variable_interpolation::{parser::Parser, VariableResolver};
    use std::collections::HashMap;

    #[test]
    fn blank_strings() {
        let res = VariableResolver::default();
        assert_eq!(Parser::start(&res, "").expect("failed"), "");
        assert_eq!(Parser::start(&res, " ").expect("failed"), " ");
    }

    #[test]
    fn without_variables() {
        let res = VariableResolver::default();
        assert_eq!(Parser::start(&res, "abc").expect("failed"), "abc");
        assert_eq!(Parser::start(&res, "a{c").expect("failed"), "a{c");
        assert_eq!(
            Parser::start(&res, "/this/is/a/path").expect("failed"),
            "/this/is/a/path"
        );
    }

    #[test]
    fn with_escaped_dollar() {
        let res = VariableResolver::default();
        assert_eq!(Parser::start(&res, "$$").expect("failed"), "$");
        assert_eq!(Parser::start(&res, "$$$$").expect("failed"), "$$");
        assert_eq!(Parser::start(&res, "$$ $$").expect("failed"), "$ $");
        assert_eq!(
            Parser::start(&res, "$$ab$$cd$$ef").expect("failed"),
            "$ab$cd$ef"
        );
    }

    #[test]
    fn test_end_braces_with_whitespace() {
        let mut res = VariableResolver::default();
        res.add_vars(HashMap::from([("KEY".into(), "VALUE".into())]));
        Parser::start(&res, "${KEY ")
            .expect_err("Closed '{' with a whitespace. Input: \"${KEY \"; got");
    }
    
    #[test]
    fn with_unterminated_brace() {
        let res = VariableResolver::default();
        let result = Parser::start(&res, "${").expect_err("did not fail");
        assert!(result.to_string().contains("input ended unexpectedly"));
    }

    #[test]
    fn with_unbound_variable_unbraced() {
        let res = VariableResolver::default();
        let result = Parser::start(&res, "$MOO").expect_err("did not fail");
        assert!(result.to_string().contains("unbound variable"));
        assert!(result.to_string().contains("MOO"));
    }

    #[test]
    fn unbraced_variable() {
        let mut res = VariableResolver::default();
        res.add_vars(HashMap::from([
            ("_".into(), "UNDERSCORE".into()),
            ("__".into(), "DOUBLESCORE".into()),
            ("a_".into(), "ALPHASCORE".into()),
        ]));
        assert_eq!(
            Parser::start(&res, "_ is $_").expect("failed"),
            "_ is UNDERSCORE"
        );
        assert_eq!(
            Parser::start(&res, "__ is $__").expect("failed"),
            "__ is DOUBLESCORE"
        );
        assert_eq!(
            Parser::start(&res, "__ is $__ and $a_").expect("failed"),
            "__ is DOUBLESCORE and ALPHASCORE"
        );
    }

    #[test]
    fn braced_variable() {
        let mut res = VariableResolver::default();
        res.add_vars(HashMap::from([
            ("_".into(), "UNDERSCORE".into()),
            ("__".into(), "DOUBLESCORE".into()),
            ("a_".into(), "ALPHASCORE".into()),
        ]));
        assert_eq!(
            Parser::start(&res, "_ is ${_}").expect("failed"),
            "_ is UNDERSCORE"
        );
        assert_eq!(
            Parser::start(&res, "__ is ${__}").expect("failed"),
            "__ is DOUBLESCORE"
        );
        assert_eq!(
            Parser::start(&res, "__ is ${__} and $a_").expect("failed"),
            "__ is DOUBLESCORE and ALPHASCORE"
        );
        assert_eq!(
            Parser::start(&res, "__ is ${ __} and $a_").expect("failed"),
            "__ is DOUBLESCORE and ALPHASCORE"
        );
    }

    #[test]
    fn braced_with_default_unset_and_empty() {
        let mut res = VariableResolver::default();
        res.add_vars(HashMap::from([("a_".into(), String::new())]));

        assert_eq!(
            Parser::start(&res, "_ is ${_:-UNDERSCORE} and more").expect("failed"),
            "_ is UNDERSCORE and more"
        );
        assert_eq!(
            Parser::start(&res, "__ is ${__:-DOUBLESCORE}").expect("failed"),
            "__ is DOUBLESCORE"
        );
        assert_eq!(
            Parser::start(&res, "__ is ${__:-DOUBLESCORE} and ${a_:-ALPHASCORE}").expect("failed"),
            "__ is DOUBLESCORE and ALPHASCORE"
        );
    }

    #[test]
    fn braced_with_default_unset_only() {
        let mut res = VariableResolver::default();
        res.add_vars(HashMap::from([("_".into(), String::new())]));

        assert_eq!(
            Parser::start(&res, "_ is ${_-UNDERSCORE} and more").expect("failed"),
            "_ is  and more"
        );

        assert_eq!(
            Parser::start(&res, "a_ is ${a_-UNDERSCORE} and more").expect("failed"),
            "a_ is UNDERSCORE and more"
        );
    }

    #[test]
    fn with_syntax_errors_incomplete_modifier() {
        let mut res = VariableResolver::default();
        res.add_vars(HashMap::from([("NAME".into(), "BRAVO".into())]));
        let result = Parser::start(&res, "${NAME:}").expect_err("did not fail");
        assert!(result.to_string().contains("expected one of ? or -"));
    }

    #[test]
    fn with_error_message_unset() {
        let res = VariableResolver::default();
        let result = Parser::start(&res, "${ANOTHERNAME??}").expect_err("did not fail");
        assert!(result.to_string().contains("(?)"));
    }

    #[test]
    fn with_error_message_empty() {
        let mut res = VariableResolver::default();
        res.add_vars(HashMap::from([("ANOTHERNAME".into(), String::new())]));
        let result = Parser::start(
            &res,
            "sometext and ${ANOTHERNAME:?ANOTHERNAME_EMPTY} or more text",
        )
        .expect_err("did not fail");
        assert!(result.to_string().contains("(ANOTHERNAME_EMPTY)"));
    }

    #[test]
    fn with_nested_variable_both_unset() {
        let res = VariableResolver::default();

        assert_eq!(
            Parser::start(&res, "${VARIABLE:-${FOO:-default}}").expect("failed"),
            "default"
        );

        assert_eq!(
            Parser::start(&res, "${VARIABLE-${FOO-default}}").expect("failed"),
            "default"
        );
    }

    #[test]
    fn with_nested_variable_second_set() {
        let mut res = VariableResolver::default();
        res.add_vars(HashMap::from([("FOO".into(), "foo".into())]));
        assert_eq!(
            Parser::start(&res, "${VARIABLE-${FOO}}").expect("failed"),
            "foo"
        );
        assert_eq!(
            Parser::start(&res, "${VARIABLE-${FOO:-default}}").expect("failed"),
            "foo"
        );
        assert_eq!(
            Parser::start(&res, "${VARIABLE-$FOO}").expect("failed"),
            "foo"
        );
    }

    #[test]
    fn with_nested_variable_second_with_error() {
        let res = VariableResolver::default();

        let result = Parser::start(
            &res,
            "sometext and ${ANOTHERNAME:-${FALLBACK:?something failed}} or more text",
        )
        .expect_err("did not fail");
        assert!(result.to_string().contains("(something failed)"));
    }

    #[test]
    fn with_nested_variables_three_levels() {
        let res = VariableResolver::default();

        assert_eq!(
            Parser::start(&res, "${VARIABLE:-${FOO:-${BAR:-third} second} first}").expect("failed"),
            "third second first"
        );
    }
}
