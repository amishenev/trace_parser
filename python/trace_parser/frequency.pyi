from __future__ import annotations

from .trace import Trace


class TraceCpuFrequency:
    base: Trace
    format_id: int
    state: int
    cpu_id: int

    @staticmethod
    def can_be_parsed(line: str) -> bool: ...

    @staticmethod
    def parse(line: str) -> TraceCpuFrequency | None: ...

    def payload_to_string(self) -> str: ...

    def to_string(self) -> str: ...


class TraceDevFrequency:
    base: Trace
    format_id: int
    clk: str
    state: int
    cpu_id: int

    @staticmethod
    def can_be_parsed(line: str) -> bool: ...

    @staticmethod
    def parse(line: str) -> TraceDevFrequency | None: ...

    def payload_to_string(self) -> str: ...

    def to_string(self) -> str: ...
