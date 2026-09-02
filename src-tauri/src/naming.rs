//! File-name patterns: `%year%/%month%/%day%/shot-%time%` and friends.
//!
//! A pattern is relative to the save folder; `/` inside it creates
//! subfolders. Tokens are case-insensitive and unknown ones are kept as
//! written so a typo is visible in the result rather than silently dropped.

use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, Datelike, Local, Timelike};

pub const DEFAULT_PATTERN: &str = "capture-%datetime%";

/// Every token with a one-line description, for the settings panel.
pub const TOKENS: &[(&str, &str)] = &[
    ("%year%", "four-digit year"),
    ("%yy%", "two-digit year"),
    ("%month%", "month, 01-12"),
    ("%monthname%", "month name, e.g. Sep"),
    ("%day%", "day of month, 01-31"),
    ("%weekday%", "weekday name, e.g. Tue"),
    ("%hour%", "hour, 00-23"),
    ("%hour12%", "hour, 01-12"),
    ("%ampm%", "AM or PM"),
    ("%minute%", "minute, 00-59"),
    ("%second%", "second, 00-59"),
    ("%ms%", "milliseconds, 000-999"),
    ("%date%", "YYYY-MM-DD"),
    ("%time%", "HH-MM-SS"),
    ("%datetime%", "YYYYMMDD-HHMMSS"),
    ("%unix%", "seconds since the epoch"),
    ("%width%", "image width in pixels"),
    ("%height%", "image height in pixels"),
    ("%random%", "six random letters and digits"),
    ("%user%", "your login name"),
    ("%n%", "smallest number that makes the name unused"),
];

#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub width: u32,
    pub height: u32,
}

/// The pattern rules the settings panel enforces before saving.
pub fn validate(pattern: &str) -> Result<(), String> {
    let trimmed = pattern.trim();
    if trimmed.is_empty() {
        return Err("file name pattern is empty".into());
    }
    let path = Path::new(trimmed);
    if path.is_absolute() || trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return Err(
            "the pattern is relative to the save folder; do not start it with a slash".into(),
        );
    }
    for component in path.components() {
        match component {
            Component::ParentDir => return Err("the pattern may not contain `..`".into()),
            Component::RootDir | Component::Prefix(_) => {
                return Err("the pattern must be a relative path".into());
            }
            _ => {}
        }
    }
    let last = trimmed.rsplit(['/', '\\']).next().unwrap_or("").trim();
    if last.is_empty() {
        return Err("the pattern must end with a file name, not a folder".into());
    }
    Ok(())
}

/// Expand every token except `%n%`, which needs the folder.
pub fn expand(pattern: &str, now: DateTime<Local>, ctx: Context, random: &str) -> String {
    let mut out = String::with_capacity(pattern.len() + 16);
    let mut rest = pattern;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let name = &after[..end];
        match token(name, now, ctx, random) {
            Some(value) => {
                out.push_str(&value);
                rest = &after[end + 1..];
            }
            None => {
                // Not a token: keep the first `%` and continue scanning after it,
                // so `100%` and `%typo` survive intact.
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn token(name: &str, now: DateTime<Local>, ctx: Context, random: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    let value = match lower.as_str() {
        "year" => format!("{:04}", now.year()),
        "yy" => format!("{:02}", now.year() % 100),
        "month" => format!("{:02}", now.month()),
        "monthname" => now.format("%b").to_string(),
        "day" => format!("{:02}", now.day()),
        "weekday" => now.format("%a").to_string(),
        "hour" => format!("{:02}", now.hour()),
        "hour12" => format!("{:02}", now.hour12().1),
        "ampm" => if now.hour12().0 { "PM" } else { "AM" }.to_string(),
        "minute" => format!("{:02}", now.minute()),
        "second" => format!("{:02}", now.second()),
        "ms" => format!("{:03}", now.timestamp_subsec_millis()),
        "date" => now.format("%Y-%m-%d").to_string(),
        "time" => now.format("%H-%M-%S").to_string(),
        "datetime" => now.format("%Y%m%d-%H%M%S").to_string(),
        "unix" => now.timestamp().to_string(),
        "width" => ctx.width.to_string(),
        "height" => ctx.height.to_string(),
        "random" => random.to_string(),
        "user" => std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "user".into()),
        "n" => "%n%".to_string(),
        _ => return None,
    };
    Some(value)
}

pub fn random_suffix() -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    (0..6)
        .map(|_| {
            let index = (rand::random::<u32>() as usize) % ALPHABET.len();
            ALPHABET[index] as char
        })
        .collect()
}

/// Characters no filesystem here accepts in a name, replaced by `_`. Path
/// separators are kept because they are the subfolder feature.
fn sanitize(expanded: &str) -> String {
    expanded
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            '\\' => '/',
            c => c,
        })
        .collect()
}

/// The full path for a new capture: expanded, sanitised, `.png` appended
/// when missing, and `%n%` resolved against what already exists.
pub fn resolve(save_dir: &Path, pattern: &str, ctx: Context) -> PathBuf {
    let now = Local::now();
    let expanded = sanitize(&expand(pattern.trim(), now, ctx, &random_suffix()));
    let with_ext = if expanded.to_ascii_lowercase().ends_with(".png") {
        expanded
    } else {
        format!("{expanded}.png")
    };
    if !with_ext.contains("%n%") {
        return save_dir.join(with_ext);
    }
    for n in 1..100_000u32 {
        let candidate = save_dir.join(with_ext.replace("%n%", &n.to_string()));
        if !candidate.exists() {
            return candidate;
        }
    }
    save_dir.join(with_ext.replace("%n%", &random_suffix()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 9, 2, 21, 5, 9).unwrap()
    }

    fn ctx() -> Context {
        Context {
            width: 1280,
            height: 720,
        }
    }

    #[test]
    fn expands_dates_and_dimensions() {
        let out = expand(
            "%year%/%month%/%day%/shot-%hour%%minute%%second%-%width%x%height%",
            at(),
            ctx(),
            "abc123",
        );
        assert_eq!(out, "2026/09/02/shot-210509-1280x720");
    }

    #[test]
    fn tokens_are_case_insensitive_and_unknown_ones_survive() {
        let out = expand("%YEAR%-%typo%-100%-%random%", at(), ctx(), "abc123");
        assert_eq!(out, "2026-%typo%-100%-abc123");
    }

    #[test]
    fn composite_tokens() {
        assert_eq!(expand("%datetime%", at(), ctx(), ""), "20260902-210509");
        assert_eq!(
            expand("%date% %time%", at(), ctx(), ""),
            "2026-09-02 21-05-09"
        );
        assert_eq!(expand("%hour12%%ampm%", at(), ctx(), ""), "09PM");
    }

    #[test]
    fn validation() {
        assert!(validate("capture-%datetime%").is_ok());
        assert!(validate("%year%/%month%/shot").is_ok());
        assert!(validate("").is_err());
        assert!(validate("/abs/path").is_err());
        assert!(validate("../escape").is_err());
        assert!(validate("%year%/").is_err());
    }

    #[test]
    fn resolve_appends_png_and_numbers() {
        let dir = std::env::temp_dir().join(format!("axio-naming-{}", random_suffix()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = resolve(&dir, "shot-%n%", ctx());
        assert_eq!(first, dir.join("shot-1.png"));
        std::fs::write(&first, b"x").unwrap();
        let second = resolve(&dir, "shot-%n%", ctx());
        assert_eq!(second, dir.join("shot-2.png"));
        let nested = resolve(&dir, "%year%/a<b>.PNG", ctx());
        assert!(nested.to_string_lossy().ends_with("/a_b_.PNG"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
