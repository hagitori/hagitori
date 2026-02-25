use hagitori_extensions::apis::date::{java_format_to_strftime, try_parse_date, parse_with_format};

#[test]
fn try_parse_iso_and_timestamps() {
    assert_eq!(try_parse_date("2024-01-15"), Some("15-01-2024".into()));
    assert_eq!(try_parse_date("2024-01-15T12:30:00Z"), Some("15-01-2024".into()));
    assert_eq!(try_parse_date("2024-01-15T12:30:00+03:00"), Some("15-01-2024".into()));
    assert_eq!(try_parse_date("1705276800"), Some("15-01-2024".into()));
    assert_eq!(try_parse_date("1705276800000"), Some("15-01-2024".into()));
}

#[test]
fn try_parse_alternative_formats() {
    assert_eq!(try_parse_date("15/01/2024"), Some("15-01-2024".into()));
    assert_eq!(try_parse_date("15.01.2024"), Some("15-01-2024".into()));
    assert_eq!(try_parse_date("January 15, 2024"), Some("15-01-2024".into()));
    assert_eq!(try_parse_date("Jan 15, 2024"), Some("15-01-2024".into()));
}

#[test]
fn try_parse_empty_returns_none() {
    assert_eq!(try_parse_date(""), None);
}

#[test]
fn parse_with_explicit_format() {
    assert_eq!(parse_with_format("01-15-2024", "MM-dd-yyyy"), Some("15-01-2024".into()));
    assert_eq!(parse_with_format("15/01/2024", "dd/MM/yyyy"), Some("15-01-2024".into()));
    assert_eq!(parse_with_format("2024-01-15 14:30:00", "yyyy-MM-dd HH:mm:ss"), Some("15-01-2024".into()));
}

#[test]
fn java_format_to_strftime_conversions() {
    assert_eq!(java_format_to_strftime("yyyy-MM-dd"), "%Y-%m-%d");
    assert_eq!(java_format_to_strftime("dd/MM/yyyy"), "%d/%m/%Y");
    assert_eq!(java_format_to_strftime("MM-dd-yyyy"), "%m-%d-%Y");
    assert_eq!(java_format_to_strftime("MMM dd, yyyy"), "%b %d, %Y");
    assert_eq!(java_format_to_strftime("MMMM dd, yyyy"), "%B %d, %Y");
    // 'm' after hour context -> minute
    assert_eq!(java_format_to_strftime("yyyy-MM-dd HH:mm:ss"), "%Y-%m-%d %H:%M:%S");
    // quoted literals
    assert_eq!(java_format_to_strftime("yyyy'T'HH:mm:ss"), "%YT%H:%M:%S");
}
