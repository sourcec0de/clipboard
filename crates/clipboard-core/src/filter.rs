//! Built-in text paste filters matching the Shortcuts & Overlay Filter action.

use serde::Deserialize;
use serde::Serialize;

/// Text transformation filter applied before pasting a clipping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TextFilter {
    /// Strip formatting and paste as plain text.
    #[default]
    PlainText,
    /// Convert text to lowercase.
    Lowercase,
    /// Convert text to UPPERCASE.
    Uppercase,
    /// Convert words to Title Case.
    TitleCase,
    /// Trim leading and trailing whitespace.
    TrimWhitespace,
    /// Replace newlines with single spaces.
    StripLineBreaks,
    /// Percent-encode URL characters.
    UrlEncode,
}

impl TextFilter {
    /// All built-in filters in display order.
    pub const ALL: &'static [Self] = &[
        Self::PlainText,
        Self::Lowercase,
        Self::Uppercase,
        Self::TitleCase,
        Self::TrimWhitespace,
        Self::StripLineBreaks,
        Self::UrlEncode,
    ];

    /// Display name shown in the Filter sheet and menus.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::PlainText => "Plain Text",
            Self::Lowercase => "Lowercase",
            Self::Uppercase => "UPPERCASE",
            Self::TitleCase => "Title Case",
            Self::TrimWhitespace => "Trim Whitespace",
            Self::StripLineBreaks => "Strip Line Breaks",
            Self::UrlEncode => "URL Encode",
        }
    }

    /// Applies this filter to `input` and returns the transformed string.
    #[must_use]
    pub fn apply(self, input: &str) -> String {
        match self {
            Self::PlainText => input.to_owned(),
            Self::Lowercase => input.to_lowercase(),
            Self::Uppercase => input.to_uppercase(),
            Self::TitleCase => to_title_case(input),
            Self::TrimWhitespace => input.trim().to_owned(),
            Self::StripLineBreaks => input
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
            Self::UrlEncode => url_encode(input),
        }
    }
}

fn to_title_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut capitalize_next = true;
    for ch in input.chars() {
        if ch.is_whitespace() || ch == '-' || ch == '_' {
            capitalize_next = true;
            out.push(ch);
        } else if capitalize_next {
            for upper in ch.to_uppercase() {
                out.push(upper);
            }
            capitalize_next = false;
        } else {
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
        }
    }
    out
}

fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            use std::fmt::Write;
            let _ = write!(&mut out, "%{byte:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::TextFilter;

    #[test]
    fn filters_transform_text_cleanly() {
        assert_eq!(TextFilter::Lowercase.apply("Hello WORLD"), "hello world");
        assert_eq!(TextFilter::Uppercase.apply("Hello world"), "HELLO WORLD");
        assert_eq!(TextFilter::TitleCase.apply("hello world"), "Hello World");
        assert_eq!(TextFilter::TrimWhitespace.apply("  hi \n"), "hi");
        assert_eq!(
            TextFilter::StripLineBreaks.apply("line one\n  line two\n\nline three"),
            "line one line two line three"
        );
        assert_eq!(TextFilter::UrlEncode.apply("a b+c"), "a%20b%2Bc");
    }
}
