from __future__ import annotations

from trace_parser import (
    Trace,
    TraceCpuFrequency,
    TraceDevFrequency,
    TraceExit,
    TraceMarkBegin,
    TraceMarkEnd,
    TraceReceiveVsync,
    TraceSchedProcessExit,
    TraceSchedSwitch,
    TraceSchedWakeup,
    TraceSchedWakeupNew,
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
    assert trace.thread_name == "bash"
    assert trace.timestamp == 12345.678901


def test_sched_switch_parse_smoke() -> None:
    line = (
        "bash-1977 (12) [000] .... 12345.678901: sched_switch: "
        "prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> "
        "next_comm=worker next_pid=123 next_prio=120"
    )
    event = TraceSchedSwitch.parse(line)
    assert event is not None


def test_macro_generated_field_access_rules() -> None:
    line = (
        "bash-1977 (12) [000] .... 12345.678901: sched_switch: "
        "prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> "
        "next_comm=worker next_pid=123 next_prio=120"
    )
    event = TraceSchedSwitch.parse(line)
    assert event is not None

    # regular field: getter + setter
    event.thread_name = "zsh"
    assert event.thread_name == "zsh"

    # readonly field: getter only
    try:
        event.event_name = "other_event"
        assert False, "event_name must be readonly"
    except AttributeError:
        pass

    # private field: not exported to Python API
    assert not hasattr(event, "format_id")


def test_sched_wakeup_parse_smoke() -> None:
    line = (
        "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: "
        "comm=bash pid=1977 prio=120 target_cpu=000"
    )
    event = TraceSchedWakeup.parse(line)
    assert event is not None
    assert event.target_cpu == 0


def test_sched_wakeup_with_reason_parse_smoke() -> None:
    line = (
        "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: "
        "comm=bash pid=1977 prio=120 target_cpu=000 reason=3"
    )
    event = TraceSchedWakeup.parse(line)
    assert event is not None
    assert event.reason == 3


def test_sched_wakeup_new_parse_smoke() -> None:
    line = (
        "kworker-123 (123) [000] .... 12345.679001: sched_wakeup_new: "
        "comm=bash pid=1977 prio=120 target_cpu=000"
    )
    event = TraceSchedWakeupNew.parse(line)
    assert event is not None
    assert event.target_cpu == 0


def test_sched_process_exit_parse_smoke() -> None:
    line = (
        "bash-1977 (12) [000] .... 12345.678901: sched_process_exit: "
        "comm=bash pid=1977 prio=120 group_dead=1"
    )
    event = TraceSchedProcessExit.parse(line)
    assert event is not None
    assert event.group_dead is True


def test_cpu_frequency_parse_smoke() -> None:
    line = (
        "swapper-0 (0) [000] .... 12345.678900: cpu_frequency: state=933000000 cpu_id=0"
    )
    event = TraceCpuFrequency.parse(line)
    assert event is not None
    assert event.state == 933000000
    assert event.cpu_id == 0


def test_dev_frequency_parse_smoke() -> None:
    line = (
        "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: "
        "clk=ddr_devfreq state=933000000 cpu_id=0"
    )
    event = TraceDevFrequency.parse(line)
    assert event is not None
    assert event.clk == "ddr_devfreq"


def test_exit1_parse_smoke() -> None:
    line = "task-100 (100) [000] .... 123.456789: exit1: pid=123 comm=test tgid=100"
    event = TraceExit.parse(line)
    assert event is not None
    assert event.pid == 123
    assert event.comm == "test"


def test_exit2_parse_smoke() -> None:
    line = "task-200 (200) [001] .... 456.789012: exit2: pid=456 comm=foo tgid=200"
    event = TraceExit.parse(line)
    assert event is not None
    assert event.pid == 456
    assert event.comm == "foo"


def test_mark_begin_parse_smoke() -> None:
    line = "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|some_message"
    event = TraceMarkBegin.parse(line)
    assert event is not None
    assert event.trace_mark_tgid == 10
    assert event.message == "some_message"
    assert event.payload == "B|10|some_message"


def test_mark_end_parse_smoke() -> None:
    line = "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: E|10|done"
    event = TraceMarkEnd.parse(line)
    assert event is not None
    assert event.trace_mark_tgid == 10
    assert event.message == "done"
    assert event.payload == "E|10|done"


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


def test_parse_dashed_tgid_as_none() -> None:
    line = (
        "<idle>-0 (-----) [001] d..2 2318.330977: sched_wakeup: "
        "comm=bash pid=1977 prio=120 target_cpu=001"
    )
    event = parse_trace(line)
    assert event is not None
    assert event.thread_tgid is None
