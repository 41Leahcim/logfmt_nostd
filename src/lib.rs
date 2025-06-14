#![no_std]

extern crate alloc;

use alloc::{
    borrow::{Cow, ToOwned},
    string::String,
    vec::Vec,
};
use core::fmt::Write as _;

#[derive(Debug)]
pub struct UnclosedString;

pub enum Token<'message> {
    Word(&'message str),
    Attribute(&'message str, &'message str),
}

impl<'message> Token<'message> {
    fn parse(s: &'message str) -> Self {
        if let Some((key, value)) = s.split_once('=') {
            let (key, value) = (key.trim(), value.trim());
            let key_length = key.chars().count();
            let value_length = value.chars().count();
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

#[derive(Debug, PartialEq, Eq)]
pub struct Log<'message> {
    message: Cow<'message, str>,
    attributes: Vec<(&'message str, &'message str)>,
}

impl Log<'_> {
    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn attributes(&self) -> &[(&str, &str)] {
        &self.attributes
    }
}

impl<'message> Log<'message> {
    pub fn parse(s: &'message str) -> Result<Self, UnclosedString> {
        let mut attributes = Vec::<(&str, &str)>::new();
        let mut chars = s.char_indices();
        let mut message = String::new();
        let mut message_property_found = false;
        loop {
            let mut in_string = false;
            let Some((start, _)) = chars.by_ref().find(|(_, ch)| !ch.is_whitespace()) else {
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
            let end = chars
                .by_ref()
                .find(|(_, c)| {
                    in_string = (in_string && *c != '"') || (!in_string && *c == '"');
                    c.is_whitespace() && !in_string
                })
                .map_or_else(|| s.len(), |(end, _)| end);

            if in_string {
                return Err(UnclosedString);
            }
            let token = &s[start..end];
            match Token::parse(token) {
                Token::Word(word) => {
                    if !message_property_found {
                        if !message.is_empty() {
                            message.push(' ');
                        }
                        write!(&mut message, "{word}").unwrap();
                    }
                }
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
