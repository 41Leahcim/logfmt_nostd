#![no_std]
#![warn(clippy::cargo)]

extern crate alloc;

use alloc::{
    borrow::{Cow, ToOwned},
    string::String,
    vec::Vec,
};
use core::fmt::Write as _;

/// An error returned when an open string is found
#[derive(Debug)]
pub struct UnclosedString;

/// A token in the log message
enum Token<'message> {
    Word(&'message str),
    Attribute(&'message str, &'message str),
}

impl<'message> Token<'message> {
    /// Parses the token from a string
    fn parse(s: &'message str) -> Self {
        // Split the message in a key and value
        // Return the message as word if not possible
        if let Some((key, value)) = s.split_once('=') {
            // Remove whitespace around the key and value
            let (key, value) = (key.trim(), value.trim());

            // Calculate the lengths of the key and value
            let key_length = key.chars().count();
            let value_length = value.chars().count();

            // Make sure the key and value are valid, otherwise return it as a word
            if (key_length > 50 && !key.starts_with('"'))
                || key_length > 52
                || key
                    .chars()
                    .take(key_length.saturating_sub(1))
                    .skip(1)
                    .any(|ch| ch == '"')
                || (key.starts_with('"') && !key.ends_with('"'))
                || (!key.starts_with('"') && key.ends_with('"'))
                || key
                    .chars()
                    .any(|ch| !ch.is_alphanumeric() && !matches!(ch, '.' | '_' | '-' | '"'))
                || (value_length > 100 && !value.starts_with('"'))
                || value_length > 102
                || value
                    .chars()
                    .take(value_length.saturating_sub(1))
                    .skip(1)
                    .any(|ch| ch == '"')
                || (value.starts_with('"') && !value.ends_with('"'))
                || (!value.starts_with('"') && value.ends_with('"'))
            {
                return Self::Word(s);
            }
            Token::Attribute(key, value)
        } else {
            Token::Word(s)
        }
    }
}

/// Contains the log message
#[derive(Debug, PartialEq, Eq)]
pub struct Log<'message> {
    message: Cow<'message, str>,
    attributes: Vec<(&'message str, &'message str)>,
}

impl Log<'_> {
    /// Return the message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Return the list of attributes
    pub fn attributes(&self) -> &[(&str, &str)] {
        &self.attributes
    }
}

impl<'message> Log<'message> {
    /// Parse the log message
    pub fn parse(s: &'message str) -> Result<Self, UnclosedString> {
        // Create a list of attributes, an iterator over the string, the message string, and a
        // variable to store whether the message property was found.
        let mut attributes = Vec::<(&str, &str)>::new();
        let mut chars = s.char_indices();
        let mut message = String::new();
        let mut message_property_found = false;

        // Iterate through the string, parsing every token.
        loop {
            // Find the start of the token.
            // Return the parse result if no token was found.
            let Some((start, _)) = chars.by_ref().find(|(_, ch)| !ch.is_whitespace()) else {
                // Store the full string as message, if no message was found.
                let message = if message.is_empty() {
                    Cow::Borrowed(s)
                } else {
                    Cow::Owned(message)
                };
                return Ok(Self {
                    message,
                    attributes,
                });
            };

            // Find the end of the token
            let mut in_string = false;
            let end = chars
                .by_ref()
                .find(|(_, c)| {
                    in_string = (in_string && *c != '"') || (!in_string && *c == '"');
                    c.is_whitespace() && !in_string
                })
                .map_or_else(|| s.len(), |(end, _)| end);

            // Return an error if a string wasn't closed.
            if in_string {
                return Err(UnclosedString);
            }

            // Parse the found token
            let token = &s[start..end];
            match Token::parse(token) {
                // If it's a word, add it to the message as a word
                Token::Word(word) => {
                    if !message_property_found {
                        if !message.is_empty() {
                            message.push(' ');
                        }
                        write!(&mut message, "{word}").unwrap();
                    }
                }

                // If it's an attribute and there is still room to add more, add it to the list
                // or assign the new value
                Token::Attribute(key, value) if attributes.len() < 25 => {
                    if matches!(key, "msg" | "message" | "\"msg\"" | "\"message\"") {
                        message = value.to_owned();
                        message_property_found = true;
                        continue;
                    }
                    match attributes
                        .iter()
                        .position(|(found_key, _)| &key == found_key)
                    {
                        Some(index) => attributes[index].1 = value,
                        None => attributes.push((key, value)),
                    }
                }
                // If there are too many attributes, add it to the message as a single word.
                _ => {
                    if !message_property_found {
                        if !message.is_empty() {
                            message.push(' ');
                        }
                        write!(&mut message, "{token}").unwrap();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::borrow::ToOwned;

    use crate::Log;

    #[test]
    fn message_with_attributes() {
        let result =
            Log::parse("this is foo=bar duration=10 a value=\"with spaces\" message").unwrap();
        assert_eq!(result.message(), "this is a message".to_owned());
        assert_eq!(
            result.attributes(),
            [
                ("foo", "bar"),
                ("duration", "10"),
                ("value", "\"with spaces\""),
            ]
        );
    }

    #[test]
    fn message_with_override() {
        let result =
            Log::parse("this is foo=bar a duration=10 message message=\"I am a message\"").unwrap();
        assert_eq!(result.message(), "\"I am a message\"");
        assert_eq!(result.attributes(), [("foo", "bar"), ("duration", "10")]);
    }

    #[test]
    fn attributes_only() {
        let message = "foo=bar duration=100";
        let result = Log::parse("foo=bar duration=100").unwrap();
        assert_eq!(result.message(), message);
        assert_eq!(result.attributes(), [("foo", "bar"), ("duration", "100")]);
    }

    #[test]
    fn webrequests() {
        for (request, attributes) in [
            "baseUrl=\"/\" hostname=localhost protocol=http",
            "baseUrl=\"/\" hostname=localhost protocol=http name=matthew",
        ]
        .into_iter()
        .zip([
            [
                ("baseUrl", "\"/\""),
                ("hostname", "localhost"),
                ("protocol", "http"),
            ]
            .as_slice(),
            [
                ("baseUrl", "\"/\""),
                ("hostname", "localhost"),
                ("protocol", "http"),
                ("name", "matthew"),
            ]
            .as_slice(),
        ]) {
            let log = Log::parse(request).unwrap();
            assert_eq!(log.message(), request);
            assert_eq!(log.attributes(), attributes);
        }
    }
}
