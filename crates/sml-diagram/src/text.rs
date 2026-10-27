//! Text measurement and XML escaping.
//!
//! Text is set in a monospace font, so its width is estimated from the character
//! count alone; no font files or shaping are needed.

/// Font size in px.
pub const FONT_SIZE: f64 = 13.0;
/// Advance width of one monospace character (0.6 em is typical for monospace fonts).
pub const CHAR_WIDTH: f64 = FONT_SIZE * 0.6;
/// Distance between consecutive text baselines.
pub const LINE_HEIGHT: f64 = 18.0;
/// Offset from a line's vertical center to its baseline.
pub const BASELINE_SHIFT: f64 = FONT_SIZE * 0.35;
/// Monospace font stack used for every text element.
pub const FONT_FAMILY: &str = "ui-monospace,SFMono-Regular,Menlo,Consolas,monospace";

/// Estimated rendered width of `text` in px.
pub fn width(text: &str) -> f64 {
    to_f64(text.chars().count()) * CHAR_WIDTH
}

/// Lossless-enough conversion for counts (saturates far beyond any real diagram).
pub fn to_f64(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}

/// Escapes `text` for use in XML character data and attribute values.
pub fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_counts_chars_not_bytes() {
        assert!((width("ab") - 2.0 * CHAR_WIDTH).abs() < 1e-9);
        assert!((width("→é") - 2.0 * CHAR_WIDTH).abs() < 1e-9);
        assert!(width("").abs() < 1e-9);
    }

    #[test]
    fn escape_covers_markup_characters() {
        assert_eq!(
            escape(r#"a<b>&"c"'d'"#),
            "a&lt;b&gt;&amp;&quot;c&quot;&apos;d&apos;"
        );
        assert_eq!(escape("plain"), "plain");
    }
}
