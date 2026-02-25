//! Date parsing utilities and Java-to-strftime format conversion for extensions.

use chrono::NaiveDate;
use rquickjs::{Ctx, Function, Value};

const AUTO_FORMATS: &[&str] = &[
    "%Y-%m-%dT%H:%M:%S%.fZ",
    "%Y-%m-%dT%H:%M:%S%.f%:z",
    "%Y-%m-%dT%H:%M:%S%:z",
    "%Y-%m-%dT%H:%M:%SZ",
    "%Y-%m-%dT%H:%M:%S",
    "%Y-%m-%dT%H:%M",
    "%Y-%m-%d",
    "%d/%m/%Y",
    "%d.%m.%Y",
    "%B %d, %Y",
    "%b %d, %Y",
    "%d %B %Y",
    "%d %b %Y",
];

/// converts a Java-style date format pattern (e.g. `"yyyy-MM-dd"`) to a `strftime` pattern.
pub fn java_format_to_strftime(java_fmt: &str) -> String {
    let mut result = String::with_capacity(java_fmt.len() * 2);
    let chars: Vec<char> = java_fmt.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        let mut count = 1;
        while i + count < len && chars[i + count] == ch {
            count += 1;
        }

        match ch {
            'y' => {
                if count >= 4 {
                    result.push_str("%Y");
                } else {
                    result.push_str("%y");
                }
            }
            'M' => {
                if count >= 4 {
                    result.push_str("%B");
                } else if count == 3 {
                    result.push_str("%b");
                } else {
                    result.push_str("%m");
                }
            }
            'd' => result.push_str("%d"),
            'H' => result.push_str("%H"),
            'h' => result.push_str("%I"),
            'm' if i > 0 && {
                let before: String = chars[..i].iter().collect();
                before.contains('H') || before.contains('h') || before.contains(':')
            } =>
            {
                result.push_str("%M")
            }
            's' => result.push_str("%S"),
            'S' => result.push_str("%.f"),
            'a' => result.push_str("%p"),
            'Z' | 'z' | 'X' => result.push_str("%:z"),
            '\'' => {
                i += 1;
                while i < len && chars[i] != '\'' {
                    result.push(chars[i]);
                    i += 1;
                }
                i += 1;
                continue;
            }
            _ => {
                for _ in 0..count {
                    result.push(ch);
                }
            }
        }

        i += count;
    }

    result
}

/// tries to parse a date without a specified format, using AUTO_FORMATS.
pub fn try_parse_date(input: &str) -> Option<String> {
    let trimmed = input.trim();

    // try numeric timestamp (ms or s)
    if let Ok(num) = trimmed.parse::<i64>() {
        let ts = if num > 1_000_000_000_000 {
            chrono::DateTime::from_timestamp(num / 1000, 0)
        } else if num > 1_000_000_000 {
            chrono::DateTime::from_timestamp(num, 0)
        } else {
            None
        };

        if let Some(dt) = ts {
            return Some(dt.format("%d-%m-%Y").to_string());
        }
    }

    for fmt in AUTO_FORMATS {
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(trimmed, fmt) {
            return Some(ndt.format("%d-%m-%Y").to_string());
        }
        if let Ok(nd) = NaiveDate::parse_from_str(trimmed, fmt) {
            return Some(nd.format("%d-%m-%Y").to_string());
        }
    }

    None
}

/// parses a date using a specific Java format.
pub fn parse_with_format(input: &str, java_fmt: &str) -> Option<String> {
    let chrono_fmt = java_format_to_strftime(java_fmt);
    let trimmed = input.trim();

    if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(trimmed, &chrono_fmt) {
        return Some(ndt.format("%d-%m-%Y").to_string());
    }
    if let Ok(nd) = NaiveDate::parse_from_str(trimmed, &chrono_fmt) {
        return Some(nd.format("%d-%m-%Y").to_string());
    }

    None
}

pub fn register<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    // parseDate(input, format?) -> "dd-MM-yyyy" | null
    globals.set(
        "parseDate",
        Function::new(ctx.clone(), |ctx: Ctx<'js>, input: String, format: rquickjs::prelude::Opt<String>| {
            let result = match format.0 {
                Some(fmt) => parse_with_format(&input, &fmt),
                None => try_parse_date(&input),
            };

            match result {
                Some(date_str) => {
                    let s = rquickjs::String::from_str(ctx.clone(), &date_str)?;
                    Ok::<_, rquickjs::Error>(s.into())
                }
                None => Ok::<_, rquickjs::Error>(Value::new_null(ctx)),
            }
        })?,
    )?;

    Ok(())
}
