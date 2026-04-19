const REDACTED_MNEMONIC: &str = "[REDACTED_MNEMONIC]";
const REDACTED_KEY_OR_SIG: &str = "[REDACTED_KEY_OR_SIG]";
const REDACTED_HEX: &str = "[REDACTED_HEX]";
const REDACTED_TOKEN: &str = "[REDACTED_TOKEN]";
const PLACEHOLDER_TOKEN: &str = "token";

#[cfg(test)]
use std::sync::{Mutex, OnceLock};

pub fn sanitize(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut index = 0;

    while index < input.len() {
        if let Some(end) = match_mnemonic(input, index) {
            output.push_str(REDACTED_MNEMONIC);
            index = end;
            continue;
        }

        if let Some(end) = match_prefixed_hex(input, index) {
            output.push_str(REDACTED_KEY_OR_SIG);
            index = end;
            continue;
        }

        if let Some(end) = match_raw_hex(input, index) {
            output.push_str(REDACTED_HEX);
            index = end;
            continue;
        }

        if let Some(end) = match_placeholder_token(input, index) {
            output.push_str(REDACTED_TOKEN);
            index = end;
            continue;
        }

        let ch = input[index..]
            .chars()
            .next()
            .expect("index always points at a char boundary");
        output.push(ch);
        index += ch.len_utf8();
    }

    output
}

pub fn sanitize_error<E: std::fmt::Display>(err: E) -> String {
    sanitize(&err.to_string())
}

#[cfg(test)]
fn test_log_buffer() -> &'static Mutex<Vec<String>> {
    static TEST_LOG_BUFFER: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    TEST_LOG_BUFFER.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(test)]
pub(crate) fn record_test_log_line(line: &str) {
    test_log_buffer().lock().unwrap().push(line.to_string());
}

#[cfg(test)]
pub(crate) fn take_test_log_lines() -> Vec<String> {
    std::mem::take(&mut *test_log_buffer().lock().unwrap())
}

#[macro_export]
macro_rules! safe_log {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        let sanitized = $crate::wallet::security::log_sanitize::sanitize(&message);
        #[cfg(test)]
        {
            let _ = ::std::io::Write::write_fmt(
                &mut ::std::io::stdout(),
                format_args!("{}\n", sanitized),
            );
            $crate::wallet::security::log_sanitize::record_test_log_line(&sanitized);
        }
        #[cfg(not(test))]
        {
            tracing::info!("{}", sanitized);
        }
    }};
}

fn match_mnemonic(input: &str, start: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    if !is_word_start(bytes, start) {
        return None;
    }

    for word_count in [24, 12] {
        let mut index = start;
        let mut matched = true;

        for current_word in 0..word_count {
            let word_start = index;
            while index < bytes.len() && is_lower_ascii(bytes[index]) {
                index += 1;
            }

            if index == word_start {
                matched = false;
                break;
            }

            if current_word + 1 < word_count {
                if index < bytes.len() && bytes[index] == b' ' {
                    index += 1;
                } else {
                    matched = false;
                    break;
                }
            }
        }

        if matched {
            return Some(index);
        }
    }

    None
}

fn match_prefixed_hex(input: &str, start: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    if start + 2 > bytes.len() || bytes[start] != b'0' || bytes[start + 1] != b'x' {
        return None;
    }

    let mut index = start + 2;
    while index < bytes.len() && bytes[index].is_ascii_hexdigit() {
        index += 1;
    }

    (index - (start + 2) >= 40).then_some(index)
}

fn match_raw_hex(input: &str, start: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    if start >= bytes.len() || !bytes[start].is_ascii_hexdigit() {
        return None;
    }

    if start > 0 && bytes[start - 1].is_ascii_hexdigit() {
        return None;
    }

    let mut index = start;
    while index < bytes.len() && bytes[index].is_ascii_hexdigit() {
        index += 1;
    }

    (index - start >= 64).then_some(index)
}

fn match_placeholder_token(input: &str, start: usize) -> Option<usize> {
    let end = start.checked_add(PLACEHOLDER_TOKEN.len())?;
    if input.get(start..end)? != PLACEHOLDER_TOKEN {
        return None;
    }

    if has_left_token_boundary(input, start) && has_right_token_boundary(input, end) {
        Some(end)
    } else {
        None
    }
}

fn is_lower_ascii(byte: u8) -> bool {
    byte.is_ascii_lowercase()
}

fn is_word_start(bytes: &[u8], start: usize) -> bool {
    start < bytes.len()
        && is_lower_ascii(bytes[start])
        && (start == 0 || !is_lower_ascii(bytes[start - 1]))
}

fn has_left_token_boundary(input: &str, index: usize) -> bool {
    input.get(..index)
        .and_then(|head| head.chars().next_back())
        .map(|ch| !is_token_char(ch))
        .unwrap_or(true)
}

fn has_right_token_boundary(input: &str, index: usize) -> bool {
    input.get(index..)
        .and_then(|tail| tail.chars().next())
        .map(|ch| !is_token_char(ch))
        .unwrap_or(true)
}

fn is_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')
}

#[cfg(test)]
mod tests {
    use super::{sanitize, sanitize_error};

    #[test]
    fn sanitize_redacts_bip39_shape() {
        let input =
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let output = sanitize(input);

        assert!(output.contains("[REDACTED_MNEMONIC]"));
        assert!(!output.contains("abandon"));
    }

    #[test]
    fn sanitize_redacts_24_word_bip39_shape() {
        let input = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

        let output = sanitize(input);

        assert_eq!(output, "[REDACTED_MNEMONIC]");
        assert!(!output.contains("abandon"));
    }

    #[test]
    fn sanitize_redacts_long_prefixed_hex() {
        let input = format!("0x{}", "a".repeat(64));

        assert_eq!(sanitize(&input), "[REDACTED_KEY_OR_SIG]");
    }

    #[test]
    fn sanitize_leaves_plain_log_line_unchanged() {
        assert_eq!(sanitize("plain log line"), "plain log line");
    }

    #[test]
    fn sanitize_redacts_raw_hex() {
        let input = "a".repeat(64);

        assert_eq!(sanitize(&input), "[REDACTED_HEX]");
    }

    #[test]
    fn sanitize_preserves_short_hex() {
        assert_eq!(
            sanitize("error: 0xdeadbeef short hex"),
            "error: 0xdeadbeef short hex"
        );
    }

    #[test]
    fn sanitize_replaces_embedded_mnemonic_inline() {
        let input = "wallet import: abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about failed";

        assert_eq!(
            sanitize(input),
            "wallet import: [REDACTED_MNEMONIC] failed"
        );
    }

    #[test]
    fn sanitize_redacts_placeholder_unlock_token() {
        assert_eq!(
            sanitize("unlock failed for token"),
            "unlock failed for [REDACTED_TOKEN]"
        );
    }

    #[test]
    fn sanitize_error_redacts_display_output() {
        let err = format!("err {}", "abandon ".repeat(11) + "about");

        assert!(sanitize_error(err).contains("[REDACTED_MNEMONIC]"));
    }

    #[test]
    fn safe_log_macro_smoke_test() {
        crate::safe_log!("logging: {}", "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about");
    }

    #[test]
    fn sanitize_is_idempotent() {
        let input = "prefix token 0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa suffix";
        let sanitized = sanitize(input);

        assert_eq!(sanitize(&sanitized), sanitized);
    }
}
