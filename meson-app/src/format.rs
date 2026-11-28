//! Plain-words formatting for sizes, times, types and costs on the file page.

use chrono::{DateTime, Utc};

/// `248311` → `248 KB`, `1204` → `1.2 KB`, `6291456` → `6.3 MB` (decimal units).
#[must_use]
#[allow(clippy::cast_precision_loss)] // file sizes far below 2^53
pub fn size_label(bytes: i64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1000 {
        return format!("{bytes} bytes");
    }
    let mut value = bytes as f64;
    let mut unit = UNITS[0];
    for u in UNITS {
        value /= 1000.0;
        unit = u;
        if value < 999.5 {
            break;
        }
    }
    if value < 9.95 {
        format!("{value:.1} {unit}")
    } else {
        format!("{value:.0} {unit}")
    }
}

/// `248311` → `248,311 bytes`.
#[must_use]
pub fn exact_bytes(bytes: i64) -> String {
    let digits = bytes.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    let sign = if bytes < 0 { "-" } else { "" };
    let noun = if bytes == 1 { "byte" } else { "bytes" };
    format!("{sign}{out} {noun}")
}

/// RFC 3339 → `Oct 24, 2026, 6:42 PM UTC`; anything unparsable comes back as is.
#[must_use]
pub fn time_label(rfc3339: &str) -> String {
    DateTime::parse_from_rfc3339(rfc3339).map_or_else(
        |_| rfc3339.to_string(),
        |t| {
            t.with_timezone(&Utc)
                .format("%b %-d, %Y, %-I:%M %p UTC")
                .to_string()
        },
    )
}

/// Whole hours between an RFC 3339 time and `now` (0 when unparsable or in the future).
#[must_use]
pub fn hours_since(rfc3339: &str, now: DateTime<Utc>) -> i64 {
    DateTime::parse_from_rfc3339(rfc3339)
        .map(|t| (now - t.with_timezone(&Utc)).num_hours().max(0))
        .unwrap_or(0)
}

/// `image/png` → `PNG image`, `text/plain` → `Text`, `application/pdf` → `PDF`.
#[must_use]
pub fn type_label(mime: &str) -> String {
    let (top, sub) = mime.split_once('/').unwrap_or((mime, ""));
    let sub = sub.split(';').next().unwrap_or("").trim();
    match (top, sub) {
        ("image", "svg+xml") => "SVG image".to_string(),
        ("image", s) if !s.is_empty() => format!("{} image", s.to_uppercase()),
        ("text", "csv") => "CSV".to_string(),
        ("text", _) => "Text".to_string(),
        ("application", "pdf") => "PDF".to_string(),
        ("application", "json") => "JSON".to_string(),
        ("application", "zip") => "Zip archive".to_string(),
        _ => mime.to_string(),
    }
}

/// `745 KB with 3 copies · under $0.01 a month`.
#[must_use]
pub fn storage_label(stored_bytes: i64, copies: i64, cost: Option<f64>, currency: &str) -> String {
    let copies_text = if copies == 1 {
        "1 copy".to_string()
    } else {
        format!("{copies} copies")
    };
    let base = format!("{} with {copies_text}", size_label(stored_bytes));
    match cost {
        Some(c) => format!("{base} · {}", cost_label(c, currency)),
        None => base,
    }
}

fn cost_label(cost: f64, currency: &str) -> String {
    let money = |v: f64| {
        if currency == "USD" {
            format!("${v:.2}")
        } else {
            format!("{v:.2} {currency}")
        }
    };
    if cost < 0.01 {
        format!("under {} a month", money(0.01))
    } else {
        format!("{} a month", money(cost))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn sizes_read_like_the_design() {
        assert_eq!(size_label(512), "512 bytes");
        assert_eq!(size_label(1_204), "1.2 KB");
        assert_eq!(size_label(248_311), "248 KB");
        assert_eq!(size_label(744_933), "745 KB");
        assert_eq!(size_label(1_873_410), "1.9 MB");
        assert_eq!(size_label(6_291_456), "6.3 MB");
        assert_eq!(size_label(999_999), "1.0 MB");
    }

    #[test]
    fn exact_bytes_groups_thousands() {
        assert_eq!(exact_bytes(1), "1 byte");
        assert_eq!(exact_bytes(999), "999 bytes");
        assert_eq!(exact_bytes(248_311), "248,311 bytes");
        assert_eq!(exact_bytes(6_291_456), "6,291,456 bytes");
    }

    #[test]
    fn times_are_utc_words() {
        assert_eq!(
            time_label("2026-10-24T18:42:07+00:00"),
            "Oct 24, 2026, 6:42 PM UTC"
        );
        assert_eq!(time_label("not a time"), "not a time");
        let now = DateTime::parse_from_rfc3339("2026-10-24T21:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(hours_since("2026-10-24T18:42:07+00:00", now), 2);
        assert_eq!(hours_since("2026-10-25T18:42:07+00:00", now), 0);
    }

    #[test]
    fn types_and_storage() {
        assert_eq!(type_label("image/png"), "PNG image");
        assert_eq!(type_label("text/plain; charset=utf-8"), "Text");
        assert_eq!(type_label("application/pdf"), "PDF");
        assert_eq!(type_label("application/x-thing"), "application/x-thing");
        assert_eq!(
            storage_label(744_933, 3, Some(0.000_01), "USD"),
            "745 KB with 3 copies · under $0.01 a month"
        );
        assert_eq!(storage_label(248_311, 1, None, "USD"), "248 KB with 1 copy");
        assert_eq!(
            storage_label(3_000_000_000, 3, Some(0.06), "EUR"),
            "3.0 GB with 3 copies · 0.06 EUR a month"
        );
    }
}
