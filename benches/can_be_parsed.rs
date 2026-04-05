use std::hint::black_box;
use std::time::Instant;

use trace_parser::{Trace, TraceSchedSwitch};

const POSITIVE_LINE: &str = "bash-1977   (  12) [000] .... 12345.678901: sched_switch: prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> next_comm=worker next_pid=123 next_prio=120";
const NEGATIVE_LINE: &str = "kworker-123 ( 123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000";

// SIMD версии (используют memchr)
fn contains_sched_switch_simd(line: &str) -> bool {
    line.contains(": sched_switch: ")
}

fn contains_sched_switch_shape_simd(line: &str) -> bool {
    line.contains(": sched_switch: ")
        && line.contains(" prev_comm=")
        && line.contains(" prev_pid=")
        && line.contains(" ==> next_comm=")
        && line.contains(" next_pid=")
}

// Версии без SIMD (для сравнения)
fn contains_sched_switch_scalar(line: &str) -> bool {
    // Скалярный поиск через str::find()
    line.find(": sched_switch: ").is_some()
}

fn contains_sched_switch_shape_scalar(line: &str) -> bool {
    line.find(": sched_switch: ").is_some()
        && line.find(" prev_comm=").is_some()
        && line.find(" prev_pid=").is_some()
        && line.find(" ==> next_comm=").is_some()
        && line.find(" next_pid=").is_some()
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
    println!("{name:<38} {ns_per_iter:>10.1} ns/op  hits={hits}/{iterations}");
}

fn main() {
    let iterations = 300_000;

    println!("positive line: {POSITIVE_LINE}");
    println!("negative line: {NEGATIVE_LINE}");
    println!();
    println!("iterations per benchmark: {iterations}");
    println!();

    println!("Positive sched_switch case");
    println!("  Fast checks (SIMD vs scalar):");
    run_bool_bench(
        "    contains() [SIMD memchr]",
        POSITIVE_LINE,
        iterations,
        contains_sched_switch_simd,
    );
    run_bool_bench(
        "    contains() [scalar find()]",
        POSITIVE_LINE,
        iterations,
        contains_sched_switch_scalar,
    );
    run_bool_bench(
        "    contains_shape() [SIMD memchr]",
        POSITIVE_LINE,
        iterations,
        contains_sched_switch_shape_simd,
    );
    run_bool_bench(
        "    contains_shape() [scalar find()]",
        POSITIVE_LINE,
        iterations,
        contains_sched_switch_shape_scalar,
    );
    println!("  Full parse (typed events):");
    run_bool_bench(
        "    Trace::can_be_parsed() [SIMD]",
        POSITIVE_LINE,
        iterations,
        Trace::can_be_parsed,
    );
    run_bool_bench(
        "    Trace::parse() [regex]",
        POSITIVE_LINE,
        iterations,
        |line| Trace::parse(line).is_some(),
    );
    run_bool_bench(
        "    TraceSchedSwitch::can_be_parsed() [SIMD]",
        POSITIVE_LINE,
        iterations,
        TraceSchedSwitch::can_be_parsed,
    );
    run_bool_bench(
        "    TraceSchedSwitch::parse() [regex]",
        POSITIVE_LINE,
        iterations,
        |line| TraceSchedSwitch::parse(line).is_some(),
    );

    println!();
    println!("Negative sched_switch case");
    println!("  Fast checks (SIMD vs scalar):");
    run_bool_bench(
        "    contains() [SIMD memchr]",
        NEGATIVE_LINE,
        iterations,
        contains_sched_switch_simd,
    );
    run_bool_bench(
        "    contains() [scalar find()]",
        NEGATIVE_LINE,
        iterations,
        contains_sched_switch_scalar,
    );
    run_bool_bench(
        "    contains_shape() [SIMD memchr]",
        NEGATIVE_LINE,
        iterations,
        contains_sched_switch_shape_simd,
    );
    run_bool_bench(
        "    contains_shape() [scalar find()]",
        NEGATIVE_LINE,
        iterations,
        contains_sched_switch_shape_scalar,
    );
    println!("  Full parse (typed events):");
    run_bool_bench(
        "    Trace::can_be_parsed() [SIMD]",
        NEGATIVE_LINE,
        iterations,
        Trace::can_be_parsed,
    );
    run_bool_bench(
        "    Trace::parse() [regex]",
        NEGATIVE_LINE,
        iterations,
        |line| Trace::parse(line).is_some(),
    );
    run_bool_bench(
        "    TraceSchedSwitch::can_be_parsed() [SIMD]",
        NEGATIVE_LINE,
        iterations,
        TraceSchedSwitch::can_be_parsed,
    );
    run_bool_bench(
        "    TraceSchedSwitch::parse() [regex]",
        NEGATIVE_LINE,
        iterations,
        |line| TraceSchedSwitch::parse(line).is_some(),
    );
}
