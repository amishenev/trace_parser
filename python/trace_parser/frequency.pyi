from __future__ import annotations

from .trace import Trace


class TraceCpuFrequency:
    base: Trace
    format_id: str
    state: int
    cpu_id: int

    @staticmethod
    def can_be_parsed(line: str) -> bool: ...

    @staticmethod
    def parse(line: str) -> TraceCpuFrequency | None: ...

    def to_string(self) -> str: ...


class TraceDevFrequency:
    base: Trace
    format_id: str
    clk: str
    state: int
    cpu_id: int

    @staticmethod
    def can_be_parsed(line: str) -> bool: ...

    @staticmethod
    def parse(line: str) -> TraceDevFrequency | None: ...

    def to_string(self) -> str: ...

