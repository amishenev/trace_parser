//! Unified throughput benchmark for trace_parser.
//!
//! Families:
//! - **core**: pure Rust parsing cost
//! - **event/<Type>/parse_only**: parse without accessing payload fields
//! - **event/<Type>/access_1**: parse + access 1 payload field
//! - **event/<Type>/access_2**: parse + access 2 payload fields
//! - **event/<Type>/access_all**: parse + access all payload fields
//! - **event/<Type>/negative**: parse non-matching line

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
// Event benchmarks (parse_only, access_1, access_2, access_all, negative)
// ---------------------------------------------------------------------------

trait BenchEvent {
    fn parse(line: &str) -> Option<Self>
    where
        Self: Sized;
    fn access_1(&self);
    fn access_2(&self);
    fn access_all(&self);
}

impl BenchEvent for TraceSchedSwitch {
    fn parse(line: &str) -> Option<Self> {
        Self::parse(line)
    }
    fn access_1(&self) {
        let _ = &self.prev_comm;
    }
    fn access_2(&self) {
        let _ = &self.prev_comm;
        let _ = &self.next_comm;
    }
    fn access_all(&self) {
        let _ = &self.prev_comm;
        let _ = &self.prev_pid;
        let _ = &self.prev_prio;
        let _ = &self.prev_state;
        let _ = &self.next_comm;
        let _ = &self.next_pid;
        let _ = &self.next_prio;
    }
}

impl BenchEvent for TraceSchedWakeup {
    fn parse(line: &str) -> Option<Self> {
        Self::parse(line)
    }
    fn access_1(&self) {
        let _ = &self.comm;
    }
    fn access_2(&self) {
        let _ = &self.comm;
        let _ = &self.pid;
    }
    fn access_all(&self) {
        let _ = &self.comm;
        let _ = &self.pid;
        let _ = &self.prio;
        let _ = &self.target_cpu;
        if let Some(s) = &self.success {
            let _ = s;
        }
        if let Some(r) = &self.reason {
            let _ = r;
        }
    }
}

impl BenchEvent for TraceDevFrequency {
    fn parse(line: &str) -> Option<Self> {
        Self::parse(line)
    }
    fn access_1(&self) {
        let _ = &self.state;
    }
    fn access_2(&self) {
        let _ = &self.state;
        let _ = &self.cpu_id;
    }
    fn access_all(&self) {
        let _ = &self.clk;
        let _ = &self.state;
        let _ = &self.cpu_id;
    }
}

impl BenchEvent for TraceMarkBegin {
    fn parse(line: &str) -> Option<Self> {
        Self::parse(line)
    }
    fn access_1(&self) {
        let _ = &self.message;
    }
    fn access_2(&self) {
        let _ = &self.message;
        let _ = &self.trace_mark_tgid;
    }
    fn access_all(&self) {
        let _ = &self.message;
        let _ = &self.trace_mark_tgid;
    }
}

impl BenchEvent for TraceReceiveVsync {
    fn parse(line: &str) -> Option<Self> {
        Self::parse(line)
    }
    fn access_1(&self) {
        let _ = &self.frame_number;
    }
    fn access_2(&self) {
        let _ = &self.frame_number;
        let _ = &self.trace_mark_tgid;
    }
    fn access_all(&self) {
        let _ = &self.frame_number;
        let _ = &self.trace_mark_tgid;
    }
}

fn bench_event_access<T: BenchEvent>(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    line: &str,
) {
    let lines: Vec<String> = (0..200).map(|_| line.to_string()).collect();
    let total_bytes: u64 = lines.iter().map(|s| s.len() as u64).sum();
    group.throughput(Throughput::Bytes(total_bytes));
    group.sample_size(50);

    group.bench_function(BenchmarkId::new(name, "parse_only"), |b| {
        b.iter(|| {
            for line in &lines {
                black_box(T::parse(black_box(line)));
            }
        });
    });

    group.bench_function(BenchmarkId::new(name, "access_1"), |b| {
        b.iter(|| {
            for line in &lines {
                if let Some(event) = T::parse(black_box(line)) {
                    black_box(event.access_1());
                }
            }
        });
    });

    group.bench_function(BenchmarkId::new(name, "access_2"), |b| {
        b.iter(|| {
            for line in &lines {
                if let Some(event) = T::parse(black_box(line)) {
                    black_box(event.access_2());
                }
            }
        });
    });

    group.bench_function(BenchmarkId::new(name, "access_all"), |b| {
        b.iter(|| {
            for line in &lines {
                if let Some(event) = T::parse(black_box(line)) {
                    black_box(event.access_all());
                }
            }
        });
    });
}

fn bench_event_negative<T: BenchEvent>(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    line: &str,
) {
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

fn bench_events(c: &mut Criterion) {
    let mut group = c.benchmark_group("event");
    group.sample_size(50);
    group.warm_up_time(std::time::Duration::from_millis(500));

    bench_event_access::<TraceSchedSwitch>(&mut group, "TraceSchedSwitch", SCHED_SWITCH_LINE);
    bench_event_negative::<TraceSchedSwitch>(&mut group, "TraceSchedSwitch", SCHED_SWITCH_NEGATIVE);

    bench_event_access::<TraceSchedWakeup>(&mut group, "TraceSchedWakeup", SCHED_WAKEUP_LINE);
    bench_event_negative::<TraceSchedWakeup>(&mut group, "TraceSchedWakeup", SCHED_WAKEUP_NEGATIVE);

    bench_event_access::<TraceDevFrequency>(&mut group, "TraceDevFrequency", DEV_FREQUENCY_LINE);
    bench_event_negative::<TraceDevFrequency>(
        &mut group,
        "TraceDevFrequency",
        DEV_FREQUENCY_NEGATIVE,
    );

    bench_event_access::<TraceMarkBegin>(&mut group, "TraceMarkBegin", MARK_BEGIN_LINE);
    bench_event_negative::<TraceMarkBegin>(&mut group, "TraceMarkBegin", MARK_BEGIN_NEGATIVE);

    bench_event_access::<TraceReceiveVsync>(&mut group, "TraceReceiveVsync", RECEIVE_VSYNC_LINE);
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
