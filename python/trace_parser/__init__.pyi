from __future__ import annotations

from typing import TypeAlias

from .frequency import TraceCpuFrequency, TraceDevFrequency
from .sched_process_exit import TraceSchedProcessExit
from .sched_switch import TraceSchedSwitch
from .sched_wakeup import TraceSchedWakeup, TraceSchedWakeupNew
from .trace import Trace
from .tracing_mark.base import TraceMarkBegin, TraceMarkEnd, TracingMark
from .tracing_mark.receive_vsync import TraceReceiveVsync

ParsedTrace: TypeAlias = (
    TraceReceiveVsync
    | TraceMarkBegin
    | TraceMarkEnd
    | TracingMark
    | TraceDevFrequency
    | TraceCpuFrequency
    | TraceSchedWakeupNew
    | TraceSchedWakeup
    | TraceSchedProcessExit
    | TraceSchedSwitch
    | Trace
)

def parse_trace(line: str) -> ParsedTrace | None: ...
def version() -> str: ...

__all__: list[str]

