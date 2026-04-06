//! SIMD-optimized base trace header parser.
//!
//! Parses `TASK-TID (TGID) [CPU] FLAGS TIMESTAMP: event_name: payload`
//! without regex, using `memchr` for field boundaries and `lexical-core` for numbers.
//!
//! Single-pass algorithm:
//! 1. Find first `": "` → separates header from event_name
//! 2. Find second `": "` → separates event_name from payload
//! 3. Parse header fields via memchr/memrchr
//! 4. Parse numbers via lexical-core (SIMD)

use lexical_core::parse;
use memchr::{memchr, memrchr};

/// Parsed base trace fields.
#[derive(Debug, Clone)]
pub struct BaseTraceParts {
    pub thread_name: String,
    pub thread_tid: u32,
    pub thread_tgid: Option<u32>,
    pub cpu: u32,
    pub flags: String,
    pub timestamp: f64,
    pub event_name: String,
    pub payload_raw: String,
}

/// Find the header/event_name boundary by validating FLAGS + TIMESTAMP.
///
/// This handles thread names containing `: `, `[`, `]`, `)`, etc.
/// Returns the offset of the `: ` that precedes the event_name.
fn find_header_boundary(bytes: &[u8]) -> Option<usize> {
    // Iterate through all ": " positions
    let mut pos = 0;
    loop {
        let colon_offset = memchr::memmem::find(&bytes[pos..], b": ")?;
        let absolute = pos + colon_offset;

        if validate_header(bytes, absolute) {
            return Some(absolute);
        }

        // Try next ": "
        pos = absolute + 2;
    }
}

/// Validate that the position before ": " is preceded by FLAGS + TIMESTAMP.
///
/// Expected pattern: `] FLAGS TIMESTAMP: `
/// where FLAGS = 3-6 chars (letters, digits, dots, dashes) and TIMESTAMP = digits.digits
fn validate_header(bytes: &[u8], colon_pos: usize) -> bool {
    if colon_pos < 8 {
        return false;
    }

    // Find last ']' before colon
    let before_colon = &bytes[..colon_pos];
    let Some(bracket_pos) = memchr::memrchr(b']', before_colon) else {
        return false;
    };
    let after_bracket = &bytes[bracket_pos + 1..colon_pos];

    // after_bracket = " .... 12345.678901" or " dn.4  2318.331005" (double space possible)
    // Need: space(s) + FLAGS (3-6 chars) + space(s) + TIMESTAMP (digits.digits)

    // Find first non-space char
    let Some(first_non_space) = after_bracket.iter().position(|&b| b != b' ') else {
        return false;
    };
    if first_non_space == 0 {
        return false; // No leading space
    }

    // Find last space after FLAGS (before TIMESTAMP)
    // Scan from end to find the last space that precedes TIMESTAMP
    let Some(last_space) = after_bracket.iter().rposition(|&b| b == b' ') else {
        return false;
    };
    if last_space <= first_non_space {
        return false;
    }

    // FLAGS = after_bracket[first_non_space..last_space]
    let flags = &after_bracket[first_non_space..last_space];
    // Trim trailing spaces from flags
    let flags_end = flags
        .iter()
        .rposition(|&b| b != b' ')
        .map(|p| p + 1)
        .unwrap_or(0);
    let flags = &flags[..flags_end];

    if flags.len() < 3 || flags.len() > 6 {
        return false;
    }
    if !flags
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
    {
        return false;
    }

    // TIMESTAMP = after_bracket[last_space+1..], trim leading spaces
    let ts_start = &after_bracket[last_space + 1..];
    let ts_start = ts_start.trim_ascii_start();
    if ts_start.is_empty() {
        return false;
    }

    let mut has_dot = false;
    let mut has_digits_before_dot = false;
    let mut has_digits_after_dot = false;

    for &b in ts_start {
        if b.is_ascii_digit() {
            if !has_dot {
                has_digits_before_dot = true;
            } else {
                has_digits_after_dot = true;
            }
        } else if b == b'.' && !has_dot && has_digits_before_dot {
            has_dot = true;
        } else {
            break;
        }
    }

    has_dot && has_digits_before_dot && has_digits_after_dot
}

impl BaseTraceParts {
    /// Parse a full trace line without regex.
    ///
    /// Returns `None` if the line doesn't match expected format.
    pub fn parse(line: &str) -> Option<Self> {
        let bytes = line.as_bytes();

        // 1. Find the header boundary (before event_name) using FLAGS+TIMESTAMP validation
        let colon1 = find_header_boundary(bytes)?;

        // 2. Find second ": " → separates event_name from payload
        let rest = &bytes[colon1 + 2..];
        let colon2 = memchr::memmem::find(rest, b": ")?;
        let event_end = colon1 + 2 + colon2;

        let event_name = std::str::from_utf8(&bytes[colon1 + 2..event_end])
            .ok()?
            .trim();
        let payload_raw = std::str::from_utf8(&bytes[event_end + 2..]).ok()?;

        // 3. Parse header: "TASK-TID (TGID) [CPU] FLAGS TIMESTAMP"
        // Search from end to handle special chars (including '(') in thread name
        let header = &bytes[..colon1];

        // Find last ')' → end of TGID (thread name may contain '(')
        let paren_close = memrchr(b')', header)?;

        // Find last '(' before ')' → start of TGID
        let paren_open = memrchr(b'(', &header[..paren_close])?;

        // Find last '-' before '(' → separates thread_name from tid
        let last_dash = memrchr(b'-', &header[..paren_open]);

        let (thread_name, tid_bytes) = if let Some(dash) = last_dash {
            (
                std::str::from_utf8(&header[..dash]).ok()?.trim(),
                &header[dash + 1..paren_open],
            )
        } else {
            // No dash — entire segment is thread_name
            ("", &header[..paren_open])
        };
        let thread_tid: u32 = {
            let s = std::str::from_utf8(tid_bytes).ok()?;
            parse(s.trim().as_bytes()).ok()?
        };

        // Parse (TGID)
        let paren_close = memchr(b')', &header[paren_open..]).map(|p| paren_open + p)?;
        let tgid_bytes = &header[paren_open + 1..paren_close];
        let tgid_str = std::str::from_utf8(tgid_bytes).ok()?.trim();
        let thread_tgid = if tgid_bytes.iter().all(|&b| b == b'-') {
            None
        } else {
            Some(parse(tgid_str.as_bytes()).ok()?)
        };

        // Parse [CPU]
        let bracket_open = memchr(b'[', &header[paren_close..]).map(|p| paren_close + p)?;
        let bracket_close = memchr(b']', &header[bracket_open..]).map(|p| bracket_open + p)?;
        let cpu_bytes = &header[bracket_open + 1..bracket_close];
        let cpu: u32 = parse(cpu_bytes).ok()?;

        // Parse FLAGS and TIMESTAMP between ] and end of header
        let after_bracket = &header[bracket_close + 1..];
        let last_space = memrchr(b' ', after_bracket)?;
        let flags = std::str::from_utf8(&after_bracket[..last_space])
            .ok()?
            .trim();
        let ts_bytes = &after_bracket[last_space + 1..];
        let timestamp: f64 = parse(ts_bytes).ok()?;

        Some(Self {
            thread_name: thread_name.to_owned(),
            thread_tid,
            thread_tgid,
            cpu,
            flags: flags.to_owned(),
            timestamp,
            event_name: event_name.to_owned(),
            payload_raw: payload_raw.to_owned(),
        })
    }
}

/// Extract event name from a trace line without full parsing.
///
/// This is used by `FastMatch::quick_check` for fast event name extraction.
pub fn extract_event_name(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    let colon1 = find_header_boundary(bytes)?;
    let rest = &bytes[colon1 + 2..];
    let colon2 = memchr::memmem::find(rest, b": ")?;
    std::str::from_utf8(&rest[..colon2]).ok().map(|s| s.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // ===== Header-only tests (parameterized) =====
    // Tests only the left part: "TASK-TID (TGID) [CPU] FLAGS TIMESTAMP:"

    #[rstest]
    #[case::simple(
        "bash-1977 (12) [000] .... 12345.678901",
        "bash",
        1977,
        Some(12),
        0,
        "....",
        12345.678901
    )]
    #[case::dashed_tgid(
        "<idle>-0 (-----) [001] dn.4 2318.331005",
        "<idle>",
        0,
        None,
        1,
        "dn.4",
        2318.331005
    )]
    #[case::dash_in_name(
        "my-thread-name-123 (456) [000] .... 100.0",
        "my-thread-name",
        123,
        Some(456),
        0,
        "....",
        100.0
    )]
    #[case::spaces_in_name(
        "my thread name-123 (456) [000] .... 100.0",
        "my thread name",
        123,
        Some(456),
        0,
        "....",
        100.0
    )]
    #[case::brackets_in_name(
        "thread[name]-123 (456) [000] .... 100.0",
        "thread[name]",
        123,
        Some(456),
        0,
        "....",
        100.0
    )]
    #[case::spaces_before_tgid(
        "app-1020-999 (  334) [000] .... 100.0",
        "app-1020",
        999,
        Some(334),
        0,
        "....",
        100.0
    )]
    #[case::parens_and_space(
        "name (inner)-123 (456) [000] .... 100.0",
        "name (inner)",
        123,
        Some(456),
        0,
        "....",
        100.0
    )]
    #[case::colon_in_name(
        "thread:name-123 (456) [000] .... 100.0",
        "thread:name",
        123,
        Some(456),
        0,
        "....",
        100.0
    )]
    #[case::unicode_name(
        "поток-123 (456) [000] .... 100.0",
        "поток",
        123,
        Some(456),
        0,
        "....",
        100.0
    )]
    #[case::empty_thread_name("-123 (456) [000] .... 100.0", "", 123, Some(456), 0, "....", 100.0)]
    #[case::special_chars(
        "th$@#!_name-999 (123) [000] .... 100.0",
        "th$@#!_name",
        999,
        Some(123),
        0,
        "....",
        100.0
    )]
    #[case::multi_digit_cpu(
        "task-100 (200) [015] .... 100.0",
        "task",
        100,
        Some(200),
        15,
        "....",
        100.0
    )]
    #[case::digits_after_dash(
        "app-123-999 (456) [000] .... 100.0",
        "app-123",
        999,
        Some(456),
        0,
        "....",
        100.0
    )]
    #[case::parens_in_name(
        "name(inner)-123 (456) [000] .... 100.0",
        "name(inner)",
        123,
        Some(456),
        0,
        "....",
        100.0
    )]
    #[case::spaces_in_tgid(
        "app-123-999 (   456) [000] .... 100.0",
        "app-123",
        999,
        Some(456),
        0,
        "....",
        100.0
    )]
    #[case::parens_in_name(
        "name(inner)-123 (456) [000] .... 100.0",
        "name(inner)",
        123,
        Some(456),
        0,
        "....",
        100.0
    )]
    #[case::colon_in_thread_name(
        "thread: name-123 (456) [000] .... 100.0",
        "thread: name",
        123,
        Some(456),
        0,
        "....",
        100.0
    )]
    #[case::brackets_in_thread_name(
        "thread[xx]: name-123 (456) [000] .... 100.0",
        "thread[xx]: name",
        123,
        Some(456),
        0,
        "....",
        100.0
    )]
    #[case::parens_and_brackets_in_thread_name(
        "custom_thread)]: name-123 (456) [000] .... 100.0",
        "custom_thread)]: name",
        123,
        Some(456),
        0,
        "....",
        100.0
    )]
    fn test_parse_header(
        #[case] header: &str,
        #[case] thread_name: &str,
        #[case] thread_tid: u32,
        #[case] thread_tgid: Option<u32>,
        #[case] cpu: u32,
        #[case] flags: &str,
        #[case] timestamp: f64,
    ) {
        let line = format!("{header}: evt: payload");
        let parts =
            BaseTraceParts::parse(&line).unwrap_or_else(|| panic!("failed to parse: {line}"));
        assert_eq!(parts.thread_name, thread_name, "thread_name");
        assert_eq!(parts.thread_tid, thread_tid, "thread_tid");
        assert_eq!(parts.thread_tgid, thread_tgid, "thread_tgid");
        assert_eq!(parts.cpu, cpu, "cpu");
        assert_eq!(parts.flags, flags, "flags");
        assert!((parts.timestamp - timestamp).abs() < 1e-6, "timestamp");
    }

    // ===== Full parse tests with real payloads =====

    #[test]
    fn test_full_parse_sched_switch() {
        let line = "bash-1977 (12) [000] .... 12345.678901: sched_switch: prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> next_comm=worker next_pid=123 next_prio=120";
        let parts = BaseTraceParts::parse(line).unwrap();
        assert_eq!(parts.event_name, "sched_switch");
        assert!(parts.payload_raw.starts_with("prev_comm=bash"));
        assert!(parts.payload_raw.contains("prev_pid=1977"));
        assert!(parts.payload_raw.contains("next_comm=worker"));
    }

    #[test]
    fn test_full_parse_sched_wakeup() {
        let line = "<idle>-0 (-----) [001] dn.4 2318.331005: sched_wakeup: comm=ksoftirqd/1 pid=12 prio=120 success=1 target_cpu=001";
        let parts = BaseTraceParts::parse(line).unwrap();
        assert_eq!(parts.event_name, "sched_wakeup");
        assert!(parts.payload_raw.contains("comm=ksoftirqd/1"));
        assert!(parts.payload_raw.contains("success=1"));
    }

    #[test]
    fn test_full_parse_sched_wakeup_with_reason() {
        let line = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000 reason=3";
        let parts = BaseTraceParts::parse(line).unwrap();
        assert_eq!(parts.event_name, "sched_wakeup");
        assert!(parts.payload_raw.contains("reason=3"));
    }

    #[test]
    fn test_full_parse_cpu_frequency() {
        let line = "swapper-0 (0) [000] .... 12345.678900: cpu_frequency: state=933000000 cpu_id=0";
        let parts = BaseTraceParts::parse(line).unwrap();
        assert_eq!(parts.event_name, "cpu_frequency");
        assert!(parts.payload_raw.contains("state=933000000"));
    }

    #[test]
    fn test_full_parse_dev_frequency() {
        let line = "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: clk=ddr_devfreq state=933000000 cpu_id=0";
        let parts = BaseTraceParts::parse(line).unwrap();
        assert_eq!(parts.event_name, "clock_set_rate");
        assert!(parts.payload_raw.contains("clk=ddr_devfreq"));
    }

    #[test]
    fn test_full_parse_tracing_mark() {
        let line = "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|[ExtraInfo]ReceiveVsync 42";
        let parts = BaseTraceParts::parse(line).unwrap();
        assert_eq!(parts.event_name, "tracing_mark_write");
        assert_eq!(parts.payload_raw, "B|10|[ExtraInfo]ReceiveVsync 42");
    }

    #[test]
    fn test_full_parse_exit1() {
        let line = "bash-1977 (12) [000] .... 12345.678901: exit1: comm=bash pid=1977 tgid=12";
        let parts = BaseTraceParts::parse(line).unwrap();
        assert_eq!(parts.event_name, "exit1");
        assert!(parts.payload_raw.contains("comm=bash"));
    }

    // ===== Unparseable cases =====

    #[test]
    fn test_unparseable_no_colon_space() {
        let line = "bash-1977 (12) [000] .... 12345.678901:sched_switch: payload";
        assert!(BaseTraceParts::parse(line).is_none());
    }

    #[test]
    fn test_unparseable_no_parens() {
        let line = "bash-1977 12 [000] .... 100.0: evt: payload";
        assert!(BaseTraceParts::parse(line).is_none());
    }

    #[test]
    fn test_extract_event_name() {
        assert_eq!(
            extract_event_name("bash-1977 (12) [000] .... 12345.678901: sched_switch: payload"),
            Some("sched_switch")
        );
    }

    #[test]
    fn test_thread_name_very_long() {
        let name = "a".repeat(100);
        let line = format!("{name}-123 (456) [000] .... 100.0: evt: payload");
        assert_eq!(BaseTraceParts::parse(&line).unwrap().thread_name, name);
    }
}
