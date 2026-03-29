from __future__ import annotations

from trace_parser import Trace, TraceDevFrequency, TraceReceiveVsync, TraceSchedSwitch, parse_trace


def handle_line(line: str) -> None:
    event: Trace | None = parse_trace(line)
    if event is None:
        return

    if isinstance(event, TraceSchedSwitch):
        print(event.prev_pid, event.next_pid)
        return

    if isinstance(event, TraceReceiveVsync):
        print(event.frame_number)
        return

    if isinstance(event, TraceDevFrequency):
        print(event.clk, event.state)
        return

    print(event.__class__.__name__)
