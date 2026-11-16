use chrono::{DateTime, Local, TimeZone};
use std::time::Duration;

/// Format a byte count using binary units, e.g. `1.24 TB`.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} B", bytes)
    } else if value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else if value >= 10.0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

/// A short byte count; always keeps two significant decimals below 10.
pub fn format_bytes_compact(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} B", bytes)
    } else if value >= 10.0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

/// Add thousands separators without relying on the system locale.
pub fn format_count(value: u64) -> String {
    let raw = value.to_string();
    let mut out = String::with_capacity(raw.len() + raw.len() / 3);
    for (idx, ch) in raw.chars().enumerate() {
        if idx > 0 && (raw.len() - idx) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

pub fn format_percent(part: u64, total: u64) -> String {
    if total == 0 || part == 0 {
        "0%".to_string()
    } else {
        let pct = part as f64 / total as f64 * 100.0;
        if pct > 0.0 && pct < 0.1 {
            "<0.1%".to_string()
        } else if pct >= 10.0 {
            format!("{pct:.0}%")
        } else {
            format!("{pct:.1}%")
        }
    }
}

pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs_f64();
    if secs < 1.0 {
        format!("{:.0} ms", secs * 1000.0)
    } else if secs < 60.0 {
        format!("{secs:.1}s")
    } else if secs < 3600.0 {
        let mins = (secs / 60.0).floor() as u64;
        let rem = secs - mins as f64 * 60.0;
        format!("{mins}m {rem:.0}s")
    } else {
        let hours = (secs / 3600.0).floor() as u64;
        let mins = ((secs - hours as f64 * 3600.0) / 60.0).floor() as u64;
        format!("{hours}h {mins}m")
    }
}

pub fn format_mtime(unix_seconds: i64) -> String {
    if unix_seconds <= 0 {
        return "—".to_string();
    }
    match Local.timestamp_opt(unix_seconds, 0).single() {
        Some(dt) => format_date(dt),
        None => "—".to_string(),
    }
}

pub fn format_date(dt: DateTime<Local>) -> String {
    dt.format("%b %-d, %Y").to_string()
}

pub fn format_datetime(unix_seconds: i64) -> String {
    if unix_seconds <= 0 {
        return "—".to_string();
    }
    match Local.timestamp_opt(unix_seconds, 0).single() {
        Some(dt) => dt.format("%b %-d, %Y · %H:%M").to_string(),
        None => "—".to_string(),
    }
}

/// Truncate the middle of a string so both the beginning and the end survive.
/// Keeps the operation UTF-8 safe.
pub fn truncate_middle(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars || max_chars < 5 {
        return text.to_string();
    }
    let keep = max_chars - 1;
    let front = keep / 2;
    let back = keep - front;
    let mut out = String::with_capacity(max_chars + 8);
    out.extend(&chars[..front]);
    out.push('…');
    out.extend(&chars[chars.len() - back..]);
    out
}

pub fn plural(value: u64, singular: &str, plural: &str) -> String {
    if value == 1 {
        format!("{} {singular}", format_count(value))
    } else {
        format!("{} {plural}", format_count(value))
    }
}

/// Convert a Windows file time (100ns since 1601) if it is useful.
pub fn filetime_to_unix(filetime: u64) -> i64 {
    if filetime < 116_444_736_000_000_000 {
        return 0;
    }
    ((filetime - 116_444_736_000_000_000) / 10_000_000) as i64
}

pub fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("USERPROFILE").map(std::path::PathBuf::from)
}

pub fn app_data_dir() -> std::path::PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .or_else(home_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let dir = base.join("G-SpaceAnalyzer");
    // Keep the settings and snapshots saved under the app's previous name.
    if !dir.exists() {
        let legacy = base.join("SpaceAnalyzer");
        if legacy.exists() && std::fs::rename(&legacy, &dir).is_err() {
            let _ = std::fs::create_dir_all(&dir);
        }
    }
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_formatting_is_readable() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(15 * 1024 * 1024), "15.0 MB");
        assert_eq!(format_bytes(1024_u64.pow(3) * 120), "120 GB");
        assert_eq!(format_bytes(1024_u64.pow(4) * 2 + 1024_u64.pow(3) * 512), "2.50 TB");
    }

    #[test]
    fn counts_have_grouping() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1_000), "1,000");
        assert_eq!(format_count(12_345_678), "12,345,678");
    }

    #[test]
    fn middle_truncation_is_utf8_safe() {
        assert_eq!(truncate_middle("short", 12), "short");
        assert_eq!(truncate_middle("a-very-long-file-name.mp4", 12), "a-ver…me.mp4");
        assert!(truncate_middle("dlouhý název souboru", 10).contains('…'));
    }

    #[test]
    fn percentages_are_sane() {
        assert_eq!(format_percent(0, 100), "0%");
        assert_eq!(format_percent(50, 100), "50%");
        assert_eq!(format_percent(1, 10_000), "<0.1%");
    }
}
