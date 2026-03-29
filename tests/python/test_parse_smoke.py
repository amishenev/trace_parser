from __future__ import annotations

from trace_parser import (
    Trace,
    TraceDevFrequency,
    TraceReceiveVsync,
    TraceSchedProcessExit,
    TraceSchedSwitch,
    TraceSchedWakeup,
    parse_trace,
)


def test_trace_parse_smoke() -> None:
    line = (
        "bash-1977 (12) [000] .... 12345.678901: sched_switch: "
        "prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> "
        "next_comm=worker next_pid=123 next_prio=120"
    )
    trace = Trace.parse(line)
    assert trace is not None
    assert trace.event_name == "sched_switch"


def test_sched_switch_parse_smoke() -> None:
    line = (
        "bash-1977 (12) [000] .... 12345.678901: sched_switch: "
        "prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> "
        "next_comm=worker next_pid=123 next_prio=120"
    )
    event = TraceSchedSwitch.parse(line)
    assert event is not None


def test_sched_wakeup_parse_smoke() -> None:
    line = (
        "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: "
        "comm=bash pid=1977 prio=120 target_cpu=000"
    )
    event = TraceSchedWakeup.parse(line)
    assert event is not None


def test_sched_process_exit_parse_smoke() -> None:
    line = (
        "bash-1977 (12) [000] .... 12345.678901: sched_process_exit: "
        "comm=bash pid=1977 prio=120 group_dead=1"
    )
    event = TraceSchedProcessExit.parse(line)
    assert event is not None


def test_dev_frequency_parse_smoke() -> None:
    line = (
        "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: "
        "clk=ddr_devfreq state=933000000 cpu_id=0"
    )
    event = TraceDevFrequency.parse(line)
    assert event is not None
    assert event.clk == "ddr_devfreq"


def test_receive_vsync_parse_smoke() -> None:
    line = (
        "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: "
        "B|10|[ExtraInfo]ReceiveVsync 42"
    )
    event = TraceReceiveVsync.parse(line)
    assert event is not None


def test_factory_parse_smoke() -> None:
    line = (
        "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: "
        "clk=l3c_devfreq state=600000000 cpu_id=0"
    )
    event = parse_trace(line)
    assert event is not None
