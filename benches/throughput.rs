//! Unified throughput benchmark for trace_parser.
//!
//! Families:
//! - **core**: pure Rust parsing cost
//! - **event**: per-event-type synthetic (positive + negative)

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use trace_parser::{
    Trace, TraceDevFrequency, TraceMarkBegin, TraceReceiveVsync, TraceSchedSwitch, TraceSchedWakeup,
};

// ---------------------------------------------------------------------------
// Sample lines for event benchmarks
// ---------------------------------------------------------------------------

const SCHED_SWITCH_LINE: &str = "bash-1977 (12) [000] .... 12345.678901: sched_switch: \
     prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> \
     next_comm=worker next_pid=123 next_prio=120";

const SCHED_SWITCH_NEGATIVE: &str = "bash-1977 (12) [000] .... 12345.678901: sched_wakeup: \
     comm=bash pid=1977 prio=120 target_cpu=000";

const SCHED_WAKEUP_LINE: &str = "<idle>-0 (-----) [001] dn.4 2318.331005: sched_wakeup: \
     comm=ksoftirqd/1 pid=12 prio=120 success=1 target_cpu=001";

const SCHED_WAKEUP_NEGATIVE: &str = "<idle>-0 (-----) [001] dn.4 2318.331005: sched_switch: \
     prev_comm=idle prev_pid=0 prev_prio=120 prev_state=R ==> \
     next_comm=ksoftirqd/1 next_pid=12 next_prio=120";

const DEV_FREQUENCY_LINE: &str = "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: \
     clk=ddr_devfreq state=933000000 cpu_id=0";

const DEV_FREQUENCY_NEGATIVE: &str = "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: \
     clk=unknown_clk state=933000000 cpu_id=0";

const MARK_BEGIN_LINE: &str = "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: \
     B|10|MyTraceEvent";

const MARK_BEGIN_NEGATIVE: &str = "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: \
     E|10|MyTraceEvent";

const RECEIVE_VSYNC_LINE: &str = "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: \
     B|10|[ExtraInfo]ReceiveVsync 42";

const RECEIVE_VSYNC_NEGATIVE: &str = "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: \
     B|10|[ExtraInfo]SomeOtherEvent 42";

// Mixed workload (realistic distribution)
const MIXED_LINES: &[&str] = &[
    SCHED_SWITCH_LINE,
    SCHED_SWITCH_LINE,
    SCHED_SWITCH_LINE,
    SCHED_WAKEUP_LINE,
    SCHED_WAKEUP_LINE,
    DEV_FREQUENCY_LINE,
    MARK_BEGIN_LINE,
    RECEIVE_VSYNC_LINE,
    SCHED_SWITCH_LINE,
    SCHED_WAKEUP_LINE,
];

// ---------------------------------------------------------------------------
// Core benchmarks
// ---------------------------------------------------------------------------

fn bench_core(c: &mut Criterion) {
    let lines: Vec<String> = MIXED_LINES
        .iter()
        .cycle()
        .take(5000)
        .cloned()
        .map(String::from)
        .collect();
    let total_bytes: u64 = lines.iter().map(|s| s.len() as u64).sum();

    let mut group = c.benchmark_group("core");
    group.throughput(Throughput::Bytes(total_bytes));
    group.sample_size(50);
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.measurement_time(std::time::Duration::from_secs(3));

    group.bench_function("rust_trace_parse", |b| {
        b.iter(|| {
            for line in &lines {
                black_box(Trace::parse(black_box(line)));
            }
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Event benchmarks (positive + negative)
// ---------------------------------------------------------------------------

fn bench_event_positive<T>(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    line: &str,
) where
    T: ParseLine,
{
    let lines: Vec<String> = (0..200).map(|_| line.to_string()).collect();
    let total_bytes: u64 = lines.iter().map(|s| s.len() as u64).sum();
    group.throughput(Throughput::Bytes(total_bytes));
    group.sample_size(50);
    group.bench_function(BenchmarkId::new(name, "positive"), |b| {
        b.iter(|| {
            for line in &lines {
                black_box(T::parse(black_box(line)));
            }
        });
    });
}

fn bench_event_negative<T>(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    line: &str,
) where
    T: ParseLine,
{
    let lines: Vec<String> = (0..200).map(|_| line.to_string()).collect();
    let total_bytes: u64 = lines.iter().map(|s| s.len() as u64).sum();
    group.throughput(Throughput::Bytes(total_bytes));
    group.sample_size(50);
    group.bench_function(BenchmarkId::new(name, "negative"), |b| {
        b.iter(|| {
            for line in &lines {
                black_box(T::parse(black_box(line)));
            }
        });
    });
}

trait ParseLine {
    fn parse(line: &str) -> Option<Self>
    where
        Self: Sized;
}

impl ParseLine for TraceSchedSwitch {
    fn parse(line: &str) -> Option<Self> {
        Self::parse(line)
    }
}

impl ParseLine for TraceSchedWakeup {
    fn parse(line: &str) -> Option<Self> {
        Self::parse(line)
    }
}

impl ParseLine for TraceDevFrequency {
    fn parse(line: &str) -> Option<Self> {
        Self::parse(line)
    }
}

impl ParseLine for TraceMarkBegin {
    fn parse(line: &str) -> Option<Self> {
        Self::parse(line)
    }
}

impl ParseLine for TraceReceiveVsync {
    fn parse(line: &str) -> Option<Self> {
        Self::parse(line)
    }
}

fn bench_events(c: &mut Criterion) {
    let mut group = c.benchmark_group("event");
    group.sample_size(50);
    group.warm_up_time(std::time::Duration::from_millis(500));

    // TraceSchedSwitch
    bench_event_positive::<TraceSchedSwitch>(&mut group, "TraceSchedSwitch", SCHED_SWITCH_LINE);
    bench_event_negative::<TraceSchedSwitch>(&mut group, "TraceSchedSwitch", SCHED_SWITCH_NEGATIVE);

    // TraceSchedWakeup
    bench_event_positive::<TraceSchedWakeup>(&mut group, "TraceSchedWakeup", SCHED_WAKEUP_LINE);
    bench_event_negative::<TraceSchedWakeup>(&mut group, "TraceSchedWakeup", SCHED_WAKEUP_NEGATIVE);

    // TraceDevFrequency
    bench_event_positive::<TraceDevFrequency>(&mut group, "TraceDevFrequency", DEV_FREQUENCY_LINE);
    bench_event_negative::<TraceDevFrequency>(
        &mut group,
        "TraceDevFrequency",
        DEV_FREQUENCY_NEGATIVE,
    );

    // TraceMarkBegin
    bench_event_positive::<TraceMarkBegin>(&mut group, "TraceMarkBegin", MARK_BEGIN_LINE);
    bench_event_negative::<TraceMarkBegin>(&mut group, "TraceMarkBegin", MARK_BEGIN_NEGATIVE);

    // TraceReceiveVsync
    bench_event_positive::<TraceReceiveVsync>(&mut group, "TraceReceiveVsync", RECEIVE_VSYNC_LINE);
    bench_event_negative::<TraceReceiveVsync>(
        &mut group,
        "TraceReceiveVsync",
        RECEIVE_VSYNC_NEGATIVE,
    );

    group.finish();
}

// ---------------------------------------------------------------------------
// Criterion groups
// ---------------------------------------------------------------------------

criterion_group!(benches, bench_core, bench_events,);
criterion_main!(benches);
