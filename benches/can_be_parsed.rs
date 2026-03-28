use std::hint::black_box;
use std::time::Instant;

use once_cell::sync::Lazy;
use regex::Regex;
use trace_parser::{Trace, TraceSchedSwitch};

static BASE_TRACE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<thread_name>.+)-(?P<tid>\d+)\s+\(\s*(?P<tgid>\d+)\)\s+\[(?P<cpu>\d+)\]\s+(?P<flags>\S+)\s+(?P<timestamp>\d+(?:\.\d+)?):\s+(?P<event_name>[^:]+):\s*(?P<payload>.*)$",
    )
    .expect("base trace regex must compile")
});

static SCHED_SWITCH_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^prev_comm=(?P<prev_comm>.+?) prev_pid=(?P<prev_pid>\d+) prev_prio=(?P<prev_prio>-?\d+) prev_state=(?P<prev_state>\S+) ==> next_comm=(?P<next_comm>.+?) next_pid=(?P<next_pid>\d+) next_prio=(?P<next_prio>-?\d+)$",
    )
    .expect("sched_switch regex must compile")
});

const POSITIVE_LINE: &str = "bash-1977   (  12) [000] .... 12345.678901: sched_switch: prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> next_comm=worker next_pid=123 next_prio=120";
const NEGATIVE_LINE: &str = "kworker-123 ( 123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000";

fn borrowed_regex_can_parse_sched_switch(line: &str) -> bool {
    let Some(captures) = BASE_TRACE_RE.captures(line) else {
        return false;
    };
    let Some(event_name) = captures.name("event_name") else {
        return false;
    };
    if event_name.as_str().trim() != "sched_switch" {
        return false;
    }
    let Some(payload) = captures.name("payload") else {
        return false;
    };
    SCHED_SWITCH_RE.is_match(payload.as_str())
}

fn contains_sched_switch(line: &str) -> bool {
    line.contains(": sched_switch: ")
}

fn contains_sched_switch_shape(line: &str) -> bool {
    line.contains(": sched_switch: ")
        && line.contains(" prev_comm=")
        && line.contains(" prev_pid=")
        && line.contains(" ==> next_comm=")
        && line.contains(" next_pid=")
}

fn run_bool_bench(name: &str, line: &str, iterations: usize, f: impl Fn(&str) -> bool) {
    for _ in 0..10_000 {
        black_box(f(black_box(line)));
    }

    let start = Instant::now();
    let mut hits = 0usize;
    for _ in 0..iterations {
        if black_box(f(black_box(line))) {
            hits += 1;
        }
    }
    let elapsed = start.elapsed();
    let ns_per_iter = elapsed.as_nanos() as f64 / iterations as f64;
    println!(
        "{name:<38} {ns_per_iter:>10.1} ns/op  hits={hits}/{iterations}"
    );
}

fn main() {
    let iterations = 300_000;

    println!("positive line: {POSITIVE_LINE}");
    println!("negative line: {NEGATIVE_LINE}");
    println!();
    println!("iterations per benchmark: {iterations}");
    println!();

    println!("Positive sched_switch case");
    run_bool_bench("Trace::can_be_parsed", POSITIVE_LINE, iterations, Trace::can_be_parsed);
    run_bool_bench(
        "Trace::parse().is_some()",
        POSITIVE_LINE,
        iterations,
        |line| Trace::parse(line).is_some(),
    );
    run_bool_bench(
        "TraceSchedSwitch::can_be_parsed",
        POSITIVE_LINE,
        iterations,
        TraceSchedSwitch::can_be_parsed,
    );
    run_bool_bench(
        "TraceSchedSwitch::parse().is_some()",
        POSITIVE_LINE,
        iterations,
        |line| TraceSchedSwitch::parse(line).is_some(),
    );
    run_bool_bench(
        "borrowed_regex_can_parse",
        POSITIVE_LINE,
        iterations,
        borrowed_regex_can_parse_sched_switch,
    );
    run_bool_bench(
        "contains(': sched_switch: ')",
        POSITIVE_LINE,
        iterations,
        contains_sched_switch,
    );
    run_bool_bench(
        "contains shape precheck",
        POSITIVE_LINE,
        iterations,
        contains_sched_switch_shape,
    );

    println!();
    println!("Negative sched_switch case");
    run_bool_bench("Trace::can_be_parsed", NEGATIVE_LINE, iterations, Trace::can_be_parsed);
    run_bool_bench(
        "Trace::parse().is_some()",
        NEGATIVE_LINE,
        iterations,
        |line| Trace::parse(line).is_some(),
    );
    run_bool_bench(
        "TraceSchedSwitch::can_be_parsed",
        NEGATIVE_LINE,
        iterations,
        TraceSchedSwitch::can_be_parsed,
    );
    run_bool_bench(
        "TraceSchedSwitch::parse().is_some()",
        NEGATIVE_LINE,
        iterations,
        |line| TraceSchedSwitch::parse(line).is_some(),
    );
    run_bool_bench(
        "borrowed_regex_can_parse",
        NEGATIVE_LINE,
        iterations,
        borrowed_regex_can_parse_sched_switch,
    );
    run_bool_bench(
        "contains(': sched_switch: ')",
        NEGATIVE_LINE,
        iterations,
        contains_sched_switch,
    );
    run_bool_bench(
        "contains shape precheck",
        NEGATIVE_LINE,
        iterations,
        contains_sched_switch_shape,
    );
}
