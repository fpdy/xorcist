//! Text utilities for display truncation.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Truncate a string to fit within a maximum display width.
/// Uses unicode-width for correct handling of CJK and other wide characters.
pub fn truncate_str(s: &str, max_width: usize) -> String {
    let width = s.width();
    if width <= max_width {
        return s.to_string();
    }

    let target_width = max_width.saturating_sub(3); // Reserve space for "..."
    let mut current_width = 0;
    let mut end_idx = 0;

    for (idx, ch) in s.char_indices() {
        let ch_width = ch.width().unwrap_or(0);
        if current_width + ch_width > target_width {
            break;
        }
        current_width += ch_width;
        end_idx = idx + ch.len_utf8();
    }

    format!("{}...", &s[..end_idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str_ascii() {
        // ASCII strings: 1 char = 1 width
        assert_eq!(truncate_str("hello world", 8), "hello...");
        assert_eq!(truncate_str("short", 10), "short");
        assert_eq!(truncate_str("exact len", 9), "exact len");
    }

    #[test]
    fn test_truncate_str_japanese() {
        // Japanese characters: 1 char = 2 width
        assert_eq!(truncate_str("日本語", 10), "日本語"); // 6 width, fits
        assert_eq!(truncate_str("日本語テスト", 10), "日本語..."); // 12 width -> truncate to 7 + "..."
    }

    #[test]
    fn test_truncate_str_mixed() {
        // Mixed ASCII and CJK
        assert_eq!(truncate_str("Hello世界", 10), "Hello世界"); // 5 + 4 = 9 width, fits
        assert_eq!(truncate_str("Hello世界!", 10), "Hello世界!"); // 5 + 4 + 1 = 10 width, fits exactly
        assert_eq!(truncate_str("Hello世界!!", 10), "Hello世..."); // 5 + 4 + 2 = 11 width, truncate
    }

    #[test]
    fn test_truncate_str_empty() {
        assert_eq!(truncate_str("", 10), "");
        assert_eq!(truncate_str("", 0), "");
    }

    #[test]
    fn test_truncate_str_small_max() {
        // When max_width is very small
        assert_eq!(truncate_str("hello", 3), "...");
        assert_eq!(truncate_str("hello", 4), "h...");
    }
}
