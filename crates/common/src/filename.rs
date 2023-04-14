use once_cell::sync::Lazy;

pub const UNICODE_REPLACEMENT_CHAR: char = '\u{fffd}';

#[cfg(unix)]
pub static FORBIDDEN_CHARS: Lazy<Vec<char>> = Lazy::new(|| vec!['/', '\0']);
#[cfg(windows)]
pub static FORBIDDEN_CHARS: Lazy<Vec<char>> = Lazy::new(|| {
    let mut chars = vec!['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    chars.extend('\0'..='\x1f'); // Control chars
    chars
});

#[cfg(unix)]
pub fn is_forbidden(ch: char) -> bool {
    matches!(ch, '/' | '\0')
}
#[cfg(windows)]
pub fn is_forbidden(ch: char) -> bool {
    matches!(
        ch,
        '\0'..='\x1f' | '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
    )
}

#[cfg(windows)]
pub const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];
