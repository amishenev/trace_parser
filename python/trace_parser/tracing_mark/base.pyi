from __future__ import annotations

from ..trace import Trace


class TracingMark:
    base: Trace

    @staticmethod
    def can_be_parsed(line: str) -> bool: ...

    @staticmethod
    def parse(line: str) -> TracingMark | None: ...

    def payload_to_string(self) -> str: ...

    def to_string(self) -> str: ...


class TraceMarkBegin:
    mark: TracingMark
    trace_mark_tgid: int
    payload: str

    @staticmethod
    def can_be_parsed(line: str) -> bool: ...

    @staticmethod
    def parse(line: str) -> TraceMarkBegin | None: ...

    def payload_to_string(self) -> str: ...

    def to_string(self) -> str: ...


class TraceMarkEnd:
    mark: TracingMark
    trace_mark_tgid: int
    payload: str

    @staticmethod
    def can_be_parsed(line: str) -> bool: ...

    @staticmethod
    def parse(line: str) -> TraceMarkEnd | None: ...

    def payload_to_string(self) -> str: ...

    def to_string(self) -> str: ...
