from __future__ import annotations

from .trace import Trace


class TraceSchedProcessExit:
    base: Trace
    format_id: str
    comm: str
    pid: int
    prio: int
    group_dead: bool

    @staticmethod
    def can_be_parsed(line: str) -> bool: ...

    @staticmethod
    def parse(line: str) -> TraceSchedProcessExit | None: ...

    def to_string(self) -> str: ...

