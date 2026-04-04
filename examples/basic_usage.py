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
    TracingMark,
    parse_trace,
    version,
)


def main() -> None:
    print(f"trace_parser version: {version()}")

    lines = [
        # sched_switch
        "bash-1977 (12) [000] .... 12345.678901: sched_switch: prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> next_comm=worker next_pid=123 next_prio=120",
        # sched_wakeup (default format)
        "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000",
        # sched_wakeup (with reason)
        "kworker-123 (123) [000] .... 12345.679002: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000 reason=3",
        # sched_wakeup_new
        "kworker-123 (123) [000] .... 12345.679003: sched_wakeup_new: comm=bash pid=1977 prio=120 target_cpu=000",
        # sched_process_exit
        "bash-1977 (12) [000] .... 12345.678901: sched_process_exit: comm=bash pid=1977 prio=120 group_dead=1",
        # exit1
        "task-100 (100) [000] .... 123.456789: exit1: pid=123 comm=test tgid=100",
        # cpu_frequency
        "swapper-0 (0) [000] .... 12345.678900: cpu_frequency: state=933000000 cpu_id=0",
        # clock_set_rate (ddr_devfreq)
        "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: clk=ddr_devfreq state=933000000 cpu_id=0",
        # tracing_mark_write: begin
        "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|some_message",
        # tracing_mark_write: end
        "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: E|10|done",
        # tracing_mark_write: ReceiveVsync
        "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|[ExtraInfo]ReceiveVsync 42",
    ]

    print("\n--- Factory parsing (parse_trace) ---")
    for line in lines:
        event = parse_trace(line)
        print(f"  {type(event).__name__}")

    print("\n--- Direct parsing with typed classes ---")

    # sched_switch
    switch = TraceSchedSwitch.parse(lines[0])
    if switch is not None:
        print(f"TraceSchedSwitch: prev={switch.prev_comm}, next={switch.next_comm}, ts={switch.timestamp}")

    # sched_wakeup (default format)
    wakeup = TraceSchedWakeup.parse(lines[1])
    if wakeup is not None:
        print(f"TraceSchedWakeup: comm={wakeup.comm}, cpu={wakeup.target_cpu}, reason={wakeup.reason}")

    # sched_wakeup (with reason)
    wakeup_reason = TraceSchedWakeup.parse(lines[2])
    if wakeup_reason is not None:
        print(f"TraceSchedWakeup (with reason): comm={wakeup_reason.comm}, reason={wakeup_reason.reason}")

    # sched_wakeup_new
    wakeup_new = TraceSchedWakeupNew.parse(lines[3])
    if wakeup_new is not None:
        print(f"TraceSchedWakeupNew: comm={wakeup_new.comm}, cpu={wakeup_new.target_cpu}")

    # sched_process_exit
    exited = TraceSchedProcessExit.parse(lines[4])
    if exited is not None:
        print(f"TraceSchedProcessExit: comm={exited.comm}, group_dead={exited.group_dead}")

    # exit1
    exited2 = TraceExit.parse(lines[5])
    if exited2 is not None:
        print(f"TraceExit: pid={exited2.pid}, comm={exited2.comm}, tgid={exited2.exit_tgid}")

    # cpu_frequency
    cpu_freq = TraceCpuFrequency.parse(lines[6])
    if cpu_freq is not None:
        print(f"TraceCpuFrequency: state={cpu_freq.state}, cpu_id={cpu_freq.cpu_id}")

    # dev_frequency
    dev_freq = TraceDevFrequency.parse(lines[7])
    if dev_freq is not None:
        print(f"TraceDevFrequency: clk={dev_freq.clk}, state={dev_freq.state}")

    # tracing_mark: begin
    begin = TraceMarkBegin.parse(lines[8])
    if begin is not None:
        print(f"TraceMarkBegin: tgid={begin.trace_mark_tgid}, message={begin.message}")

    # tracing_mark: end
    end = TraceMarkEnd.parse(lines[9])
    if end is not None:
        print(f"TraceMarkEnd: tgid={end.trace_mark_tgid}, message={end.message}")

    # tracing_mark: ReceiveVsync
    vsync = TraceReceiveVsync.parse(lines[10])
    if vsync is not None:
        print(f"TraceReceiveVsync: frame={vsync.frame_number}")

    print("\n--- Round-trip (to_string / parse) ---")
    if switch is not None:
        rendered = switch.to_string()
        reparsed = TraceSchedSwitch.parse(rendered)
        if reparsed is not None:
            print(f"  sched_switch round-trip: {reparsed.prev_comm} -> {reparsed.next_comm}")

    if wakeup_reason is not None:
        rendered = wakeup_reason.to_string()
        reparsed = TraceSchedWakeup.parse(rendered)
        if reparsed is not None:
            print(f"  sched_wakeup (with reason) round-trip: reason={reparsed.reason}")

    if dev_freq is not None:
        rendered = dev_freq.to_string()
        reparsed = TraceDevFrequency.parse(rendered)
        if reparsed is not None:
            print(f"  dev_frequency round-trip: clk={reparsed.clk}")


if __name__ == "__main__":
    main()
