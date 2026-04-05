"""trace_parser — fast ftrace/tracefs log parser."""

from trace_parser._native import (
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
    parse_trace_file,
    version,
)

__all__ = (
    "Trace",
    "TraceCpuFrequency",
    "TraceDevFrequency",
    "TraceExit",
    "TraceMarkBegin",
    "TraceMarkEnd",
    "TraceReceiveVsync",
    "TraceSchedProcessExit",
    "TraceSchedSwitch",
    "TraceSchedWakeup",
    "TraceSchedWakeupNew",
    "TracingMark",
    "parse_trace",
    "parse_trace_file",
    "version",
)
