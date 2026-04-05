from __future__ import annotations

import tempfile
from pathlib import Path

from trace_parser import parse_trace_file


def test_parse_trace_file() -> None:
    """Test basic parse_trace_file functionality."""
    # Create a temporary trace file
    trace_lines = [
        "bash-1977 (12) [000] .... 12345.678901: sched_switch: "
        "prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> "
        "next_comm=worker next_pid=123 next_prio=120",
        "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: "
        "comm=bash pid=1977 prio=120 target_cpu=000",
        "bash-1977 (12) [000] .... 12345.678902: sched_process_exit: "
        "comm=bash pid=1977 prio=120 group_dead=0",
    ]

    with tempfile.NamedTemporaryFile(mode="w", suffix=".trace", delete=False) as f:
        f.write("\n".join(trace_lines))
        temp_path = Path(f.name)

    try:
        # Parse entire file
        events = parse_trace_file(str(temp_path))
        assert len(events) == 3

        # Check event types
        from trace_parser import (
            TraceSchedSwitch,
            TraceSchedWakeup,
            TraceSchedProcessExit,
        )

        assert isinstance(events[0], TraceSchedSwitch)
        assert isinstance(events[1], TraceSchedWakeup)
        assert isinstance(events[2], TraceSchedProcessExit)

        # Parse with filter
        events_filtered = parse_trace_file(str(temp_path), filter_event="sched_switch")
        assert len(events_filtered) == 1
        assert isinstance(events_filtered[0], TraceSchedSwitch)

    finally:
        temp_path.unlink()


def test_parse_trace_file_empty() -> None:
    """Test parse_trace_file with empty file."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".trace", delete=False) as f:
        temp_path = Path(f.name)

    try:
        events = parse_trace_file(str(temp_path))
        assert len(events) == 0
    finally:
        temp_path.unlink()


def test_parse_trace_file_filter_no_matches() -> None:
    """Test parse_trace_file with filter that has no matches."""
    trace_line = (
        "bash-1977 (12) [000] .... 12345.678901: sched_switch: "
        "prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> "
        "next_comm=worker next_pid=123 next_prio=120"
    )

    with tempfile.NamedTemporaryFile(mode="w", suffix=".trace", delete=False) as f:
        f.write(trace_line)
        temp_path = Path(f.name)

    try:
        events = parse_trace_file(str(temp_path), filter_event="nonexistent_event")
        assert len(events) == 0
    finally:
        temp_path.unlink()
