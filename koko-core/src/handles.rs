//! Identity rules: Koko users are **plain usernames** — no email, no phone
//! number, no domain.

use std::fmt;

/// Errors from handle/display-name validation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HandleError {
    /// Wrong length (must be 3–24 characters).
    #[error("handle must be 3-24 characters")]
    Length,
    /// First character must be a lowercase letter.
    #[error("handle must start with a lowercase letter")]
    FirstChar,
    /// Only `a-z`, `0-9` and `_` after the first character.
    #[error("handle may only contain a-z, 0-9 and _")]
    Charset,
    /// Reserved name (cannot be registered).
    #[error("handle is reserved")]
    Reserved,
}

/// The handle grammar: `^[a-z][a-z0-9_]{2,23}$`.
///
/// 3–24 characters, starts with a lowercase letter, continues with lowercase
/// letters, digits or underscores. Uppercase is intentionally rejected —
/// handles are canonical, which avoids `Mira`/`mira` confusion (SQLite also
/// enforces case-insensitive uniqueness as a belt-and-braces measure).
const RESERVED: [&str; 7] = [
    "admin", "root", "system", "koko", "support", "security", "nobody",
];

pub fn validate_handle(handle: &str) -> Result<(), HandleError> {
    let re = regex::Regex::new(r"^[a-z][a-z0-9_]{2,23}$").expect("static regex compiles");
    if !re.is_match(handle) {
        // Distinguish the failure modes for a helpful error message.
        let len = handle.chars().count();
        if !(3..=24).contains(&len) {
            return Err(HandleError::Length);
        }
        let mut chars = handle.chars();
        match chars.next() {
            Some(c) if c.is_ascii_lowercase() => {}
            _ => return Err(HandleError::FirstChar),
        }
        return Err(HandleError::Charset);
    }
    if RESERVED.contains(&handle) {
        return Err(HandleError::Reserved);
    }
    Ok(())
}

/// Display names: 1–64 characters, no control characters.
pub fn validate_display_name(name: &str) -> Result<(), HandleError> {
    let len = name.chars().count();
    if !(1..=64).contains(&len) {
        return Err(HandleError::Length);
    }
    if name.chars().any(char::is_control) {
        return Err(HandleError::Charset);
    }
    Ok(())
}

/// A validated handle (parse-don't-validate helper).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Handle(String);

impl Handle {
    /// Validate and wrap a handle.
    pub fn parse(raw: &str) -> Result<Self, HandleError> {
        validate_handle(raw)?;
        Ok(Handle(raw.to_owned()))
    }

    /// The handle string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_good_handles() {
        for h in [
            "mira",
            "kai",
            "abc",
            "a_b_c",
            "user123",
            "x".repeat(24).as_str(),
        ] {
            assert!(validate_handle(h).is_ok(), "{h} should be valid");
        }
    }

    #[test]
    fn rejects_bad_handles() {
        for h in [
            "ab",                        // too short
            "Mira",                      // uppercase
            "1mira",                     // starts with digit
            "_mira",                     // starts with underscore
            "mira!",                     // bad charset
            "mira moore",                // space
            "aaaaaaaaaaaaaaaaaaaaaaaaa", // 25 chars
            "",                          // empty
        ] {
            assert!(validate_handle(h).is_err(), "{h} should be invalid");
        }
    }

    #[test]
    fn rejects_reserved_handles() {
        assert_eq!(validate_handle("admin"), Err(HandleError::Reserved));
    }

    #[test]
    fn display_names_bounds() {
        assert!(validate_display_name("Mira Okafor").is_ok());
        assert!(validate_display_name("").is_err());
        assert!(validate_display_name(&"x".repeat(65)).is_err());
    }
}
