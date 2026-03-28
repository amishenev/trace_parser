from __future__ import annotations

from .trace import Trace


class TraceSchedSwitch:
    base: Trace
    format_id: str
    prev_comm: str
    prev_pid: int
    prev_prio: int
    prev_state: str
    next_comm: str
    next_pid: int
    next_prio: int

    @staticmethod
    def can_be_parsed(line: str) -> bool: ...

    @staticmethod
    def parse(line: str) -> TraceSchedSwitch | None: ...

    def payload_to_string(self) -> str: ...

    def to_string(self) -> str: ...
