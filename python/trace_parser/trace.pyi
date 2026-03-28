from __future__ import annotations


class Trace:
    thread_name: str
    tid: int
    tgid: int
    cpu: int
    flags: str
    timestamp: float
    event_name: str
    payload_raw: str

    @staticmethod
    def can_be_parsed(line: str) -> bool: ...

    @staticmethod
    def parse(line: str) -> Trace | None: ...

    @property
    def timestamp_ms(self) -> float: ...

    @timestamp_ms.setter
    def timestamp_ms(self, value: float) -> None: ...

    @property
    def timestamp_ns(self) -> int: ...

    @timestamp_ns.setter
    def timestamp_ns(self, value: int) -> None: ...

    def to_string(self) -> str: ...

