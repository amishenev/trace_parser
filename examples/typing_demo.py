"""
Type-safe event handling demo.

Shows how to use isinstance() checks and type hints
for working with different trace event types.
"""
from __future__ import annotations

from trace_parser import (
    Trace,
    TraceDevFrequency,
    TraceMarkBegin,
    TraceMarkEnd,
    TraceReceiveVsync,
    TraceSchedSwitch,
    TraceSchedWakeup,
    parse_trace,
)


def handle_line(line: str) -> None:
    """Parse and process a single trace line with type-safe dispatching."""
    event: Trace | None = parse_trace(line)
    if event is None:
        print(f"  [unknown] {line[:60]}...")
        return

    # Type-safe dispatching via isinstance()
    if isinstance(event, TraceSchedSwitch):
        print(f"  [sched_switch] {event.prev_comm} (pid={event.prev_pid}) -> {event.next_comm} (pid={event.next_pid})")
        return

    if isinstance(event, TraceSchedWakeup):
        reason_str = f" reason={event.reason}" if event.reason is not None else ""
        print(f"  [sched_wakeup] {event.comm} (pid={event.pid}) cpu={event.target_cpu}{reason_str}")
        return

    if isinstance(event, TraceDevFrequency):
        print(f"  [dev_frequency] clk={event.clk} state={event.state}")
        return

    if isinstance(event, TraceReceiveVsync):
        print(f"  [receive_vsync] frame={event.frame_number}")
        return

    if isinstance(event, TraceMarkBegin):
        print(f"  [mark_begin] tgid={event.trace_mark_tgid} message={event.message}")
        return

    if isinstance(event, TraceMarkEnd):
        print(f"  [mark_end] tgid={event.trace_mark_tgid} message={event.message}")
        return

    # Fallback for unhandled types
    print(f"  [{type(event).__name__}] {event.event_name}")


def main() -> None:
    lines = [
        "bash-1977 (12) [000] .... 12345.678901: sched_switch: prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> next_comm=worker next_pid=123 next_prio=120",
        "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000",
        "kworker-123 (123) [000] .... 12345.679002: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000 reason=3",
        "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: clk=ddr_devfreq state=933000000 cpu_id=0",
        "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|[ExtraInfo]ReceiveVsync 42",
        "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|custom_begin",
        "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: E|10|custom_end",
    ]

    print("Processing trace lines:")
    for line in lines:
        handle_line(line)


if __name__ == "__main__":
    main()
