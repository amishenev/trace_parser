from __future__ import annotations

from .base import TraceMarkBegin


class TraceReceiveVsync:
    begin: TraceMarkBegin
    frame_number: int

    @staticmethod
    def can_be_parsed(line: str) -> bool: ...

    @staticmethod
    def parse(line: str) -> TraceReceiveVsync | None: ...

    def payload_to_string(self) -> str: ...

    def to_string(self) -> str: ...
