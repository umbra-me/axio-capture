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

/// The pattern rules the settings panel enforces before saving. `\` is
/// turned into `/` first, as `sanitize` does when the name is built, so a
/// separator cannot hide a `..` from this check on any platform. `finish`
/// checks the expanded name again.
pub fn validate(pattern: &str) -> Result<(), String> {
    check_relative(&pattern.trim().replace('\\', "/"))
}

/// A name with `/` separators must stay inside the save folder and end in a
/// file name.
fn check_relative(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("file name pattern is empty".into());
    }
    let path = Path::new(name);
    if path.is_absolute() || name.starts_with('/') {
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
            // Windows drops trailing dots and spaces from a name, which
            // would turn `.. ` or `...` into `..`.
            Component::Normal(part)
                if part.to_str().is_some_and(|part| {
                    part.chars().all(|c| c == '.' || c == ' ') && part.matches('.').count() >= 2
                }) =>
            {
                return Err("the pattern may not contain `..`".into());
            }
            _ => {}
        }
    }
    let last = name.rsplit('/').next().unwrap_or("").trim();
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

/// An expanded pattern as a relative file name: sanitised, checked again
/// (tokens such as `%user%` come from the environment, so the pattern check
/// alone cannot vouch for the result) and `.png` appended when missing.
fn finish(expanded: &str) -> Result<String, String> {
    let name = sanitize(expanded);
    check_relative(&name)?;
    if name.to_ascii_lowercase().ends_with(".png") {
        Ok(name)
    } else {
        Ok(format!("{name}.png"))
    }
}

/// The names a new capture may take, best first, expanded once so every
/// candidate shares one timestamp. `%n%` counts up from 1; a pattern without
/// it is tried as written and then with `-2`, `-3`, ... before the
/// extension, so a fixed pattern or two captures in the same second never
/// share a file.
pub fn candidates(
    save_dir: &Path,
    pattern: &str,
    ctx: Context,
) -> Result<impl Iterator<Item = PathBuf> + use<>, String> {
    let name = finish(&expand(pattern.trim(), Local::now(), ctx, &random_suffix()))?;
    let save_dir = save_dir.to_path_buf();
    Ok((1..=MAX_CANDIDATES).map(move |n| save_dir.join(numbered(&name, n))))
}

const MAX_CANDIDATES: u32 = 100_000;

fn numbered(name: &str, n: u32) -> String {
    if name.contains("%n%") {
        name.replace("%n%", &n.to_string())
    } else if n == 1 {
        name.to_string()
    } else {
        // `finish` guarantees an ASCII `.png` ending.
        let (stem, extension) = name.split_at(name.len() - ".png".len());
        format!("{stem}-{n}{extension}")
    }
}

/// The first candidate nothing occupies yet, for the settings preview and the
/// Save As suggestion. It reserves nothing; `export::save_new` does that.
pub fn resolve(save_dir: &Path, pattern: &str, ctx: Context) -> Result<PathBuf, String> {
    candidates(save_dir, pattern, ctx)?
        .find(|candidate| std::fs::symlink_metadata(candidate).is_err())
        .ok_or_else(|| "every numbered name for this pattern is taken".into())
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
    fn validation_normalises_backslashes_before_looking_for_parents() {
        // `\` becomes `/` when the name is built, so it must count as one here.
        assert!(validate("..\\outside").is_err());
        assert!(validate("shots\\..\\..\\outside").is_err());
        assert!(validate("\\outside").is_err());
        assert!(validate("%year%\\shot").is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn validation_still_rejects_a_drive_path_on_windows() {
        // Sanitising would turn `C:` into a harmless `C_` folder; the user
        // typed an absolute path and should be told so.
        assert!(validate("C:\\shots\\x").is_err());
        assert!(validate("C:/shots/x").is_err());
    }

    #[test]
    fn validation_rejects_dot_only_components() {
        // Windows drops trailing dots and spaces, which turns these into `..`.
        assert!(validate("... /shot").is_err());
        assert!(validate("shots/.. /shot").is_err());
        assert!(validate("./shot").is_ok());
        assert!(validate(".hidden").is_ok());
    }

    #[test]
    fn the_expanded_name_is_checked_again() {
        // Tokens such as `%user%` come from the environment, so the pattern
        // check alone cannot vouch for the final name.
        assert!(finish("..\\outside").is_err());
        assert!(finish("/etc/shot").is_err());
        assert!(finish("2026/..").is_err());
        assert_eq!(finish("2026\\shot").unwrap(), "2026/shot.png");
        assert_eq!(finish("a<b>.PNG").unwrap(), "a_b_.PNG");
    }

    #[test]
    fn resolve_appends_png_and_numbers() {
        let dir = std::env::temp_dir().join(format!("axio-naming-{}", random_suffix()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = resolve(&dir, "shot-%n%", ctx()).unwrap();
        assert_eq!(first, dir.join("shot-1.png"));
        std::fs::write(&first, b"x").unwrap();
        let second = resolve(&dir, "shot-%n%", ctx()).unwrap();
        assert_eq!(second, dir.join("shot-2.png"));
        let nested = resolve(&dir, "%year%/a<b>.PNG", ctx()).unwrap();
        assert!(nested.to_string_lossy().ends_with("/a_b_.PNG"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resolve_never_offers_a_taken_name_for_a_fixed_pattern() {
        let dir = std::env::temp_dir().join(format!("axio-naming-{}", random_suffix()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("shot.png"), b"x").unwrap();
        assert_eq!(
            resolve(&dir, "shot", ctx()).unwrap(),
            dir.join("shot-2.png")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn candidates_number_before_the_extension_or_at_n() {
        let dir = Path::new("/captures");
        let fixed: Vec<_> = candidates(dir, "shot.PNG", ctx())
            .unwrap()
            .take(3)
            .collect();
        assert_eq!(
            fixed,
            [
                dir.join("shot.PNG"),
                dir.join("shot-2.PNG"),
                dir.join("shot-3.PNG")
            ]
        );
        let numbered: Vec<_> = candidates(dir, "%n%/shot", ctx())
            .unwrap()
            .take(2)
            .collect();
        assert_eq!(numbered, [dir.join("1/shot.png"), dir.join("2/shot.png")]);
        assert!(candidates(dir, "..\\outside", ctx()).is_err());
    }
}
