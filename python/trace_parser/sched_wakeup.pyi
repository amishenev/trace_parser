from __future__ import annotations

from .trace import Trace


class TraceSchedWakeup:
    base: Trace
    format_id: str
    comm: str
    pid: int
    prio: int
    target_cpu: int

    @staticmethod
    def can_be_parsed(line: str) -> bool: ...

    @staticmethod
    def parse(line: str) -> TraceSchedWakeup | None: ...

    def to_string(self) -> str: ...


class TraceSchedWakeupNew:
    base: Trace
    format_id: str
    comm: str
    pid: int
    prio: int
    target_cpu: int

    @staticmethod
    def can_be_parsed(line: str) -> bool: ...

    @staticmethod
    def parse(line: str) -> TraceSchedWakeupNew | None: ...

    def to_string(self) -> str: ...

