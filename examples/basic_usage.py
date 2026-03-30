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


def main() -> None:
    lines = [
        "bash-1977 (12) [000] .... 12345.678901: sched_switch: prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> next_comm=worker next_pid=123 next_prio=120",
        "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000",
        "bash-1977 (12) [000] .... 12345.678901: sched_process_exit: comm=bash pid=1977 prio=120 group_dead=1",
        "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: clk=ddr_devfreq state=933000000 cpu_id=0",
        "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|[ExtraInfo]ReceiveVsync 42",
    ]

    for line in lines:
        event = parse_trace(line)
        print(type(event).__name__ if event is not None else "None")

    switch = TraceSchedSwitch.parse(lines[0])
    if switch is not None:
        print("switch", switch.prev_comm, switch.next_comm, switch.timestamp)

    wakeup = TraceSchedWakeup.parse(lines[1])
    if wakeup is not None:
        print("wakeup", wakeup.comm, wakeup.target_cpu)

    exited = TraceSchedProcessExit.parse(lines[2])
    if exited is not None:
        print("exit", exited.comm, exited.group_dead)

    devfreq = TraceDevFrequency.parse(lines[3])
    if devfreq is not None:
        print("devfreq", devfreq.clk, devfreq.state)

    vsync = TraceReceiveVsync.parse(lines[4])
    if vsync is not None:
        print("vsync", vsync.frame_number, vsync.begin.trace_mark_tgid)

    vsync_from_module = TraceReceiveVsyncModule.parse(lines[4])
    if vsync_from_module is not None:
        print("vsync-module", vsync_from_module.frame_number)

    trace = Trace.parse(lines[0])
    if trace is not None:
        print("raw", trace.event_name, trace.timestamp_ms)


if __name__ == "__main__":
    main()
