"""Type stubs for trace_parser._native native extension."""

from __future__ import annotations
from typing_extensions import Self

class Trace:
    thread_name: str
    thread_tid: int
    thread_tgid: int | None
    cpu: int
    flags: str
    timestamp: float
    event_name: str
    payload_raw: str
    def __init__(
        self,
        thread_name: str,
        thread_tid: int,
        thread_tgid: int | None,
        cpu: int,
        flags: str,
        timestamp: float,
        event_name: str,
        payload_raw: str,
    ) -> None: ...
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
    @property
    def payload(self) -> str: ...
    @property
    def template(self) -> str: ...
    def payload_to_string(self) -> str: ...
    def to_string(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> Trace: ...
    def __deepcopy__(self, memo: object) -> Trace: ...

class TraceCpuFrequency:
    thread_name: str
    thread_tid: int
    thread_tgid: int | None
    cpu: int
    flags: str
    timestamp: float
    event_name: str
    state: int
    cpu_id: int
    def __init__(
        self,
        thread_name: str,
        thread_tid: int,
        thread_tgid: int | None,
        cpu: int,
        flags: str,
        timestamp: float,
        event_name: str,
        state: int,
        cpu_id: int,
    ) -> None: ...
    @staticmethod
    def can_be_parsed(line: str) -> bool: ...
    @staticmethod
    def parse(line: str) -> Self | None: ...
    @property
    def payload(self) -> str: ...
    @property
    def template(self) -> str: ...
    def to_string(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> Self: ...
    def __deepcopy__(self, memo: object) -> Self: ...

class TraceDevFrequency:
    thread_name: str
    thread_tid: int
    thread_tgid: int | None
    cpu: int
    flags: str
    timestamp: float
    event_name: str
    clk: str
    state: int
    cpu_id: int
    def __init__(
        self,
        thread_name: str,
        thread_tid: int,
        thread_tgid: int | None,
        cpu: int,
        flags: str,
        timestamp: float,
        event_name: str,
        clk: str,
        state: int,
        cpu_id: int,
    ) -> None: ...
    @staticmethod
    def can_be_parsed(line: str) -> bool: ...
    @staticmethod
    def parse(line: str) -> Self | None: ...
    @property
    def payload(self) -> str: ...
    @property
    def template(self) -> str: ...
    def to_string(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> Self: ...
    def __deepcopy__(self, memo: object) -> Self: ...

class TraceExit:
    thread_name: str
    thread_tid: int
    thread_tgid: int | None
    cpu: int
    flags: str
    timestamp: float
    event_name: str
    pid: int
    comm: str
    tgid: int
    def __init__(
        self,
        thread_name: str,
        thread_tid: int,
        thread_tgid: int | None,
        cpu: int,
        flags: str,
        timestamp: float,
        event_name: str,
        pid: int,
        comm: str,
        tgid: int,
    ) -> None: ...
    @staticmethod
    def can_be_parsed(line: str) -> bool: ...
    @staticmethod
    def parse(line: str) -> Self | None: ...
    @property
    def payload(self) -> str: ...
    @property
    def template(self) -> str: ...
    def to_string(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> Self: ...
    def __deepcopy__(self, memo: object) -> Self: ...

class TraceMarkBegin:
    thread_name: str
    thread_tid: int
    thread_tgid: int | None
    cpu: int
    flags: str
    timestamp: float
    event_name: str
    trace_mark_tgid: int
    message: str
    def __init__(
        self,
        thread_name: str,
        thread_tid: int,
        thread_tgid: int | None,
        cpu: int,
        flags: str,
        timestamp: float,
        event_name: str,
        trace_mark_tgid: int,
        message: str,
    ) -> None: ...
    @staticmethod
    def can_be_parsed(line: str) -> bool: ...
    @staticmethod
    def parse(line: str) -> Self | None: ...
    @property
    def payload(self) -> str: ...
    @property
    def template(self) -> str: ...
    def to_string(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> Self: ...
    def __deepcopy__(self, memo: object) -> Self: ...

class TraceMarkEnd:
    thread_name: str
    thread_tid: int
    thread_tgid: int | None
    cpu: int
    flags: str
    timestamp: float
    event_name: str
    trace_mark_tgid: int
    message: str
    def __init__(
        self,
        thread_name: str,
        thread_tid: int,
        thread_tgid: int | None,
        cpu: int,
        flags: str,
        timestamp: float,
        event_name: str,
        trace_mark_tgid: int,
        message: str,
    ) -> None: ...
    @staticmethod
    def can_be_parsed(line: str) -> bool: ...
    @staticmethod
    def parse(line: str) -> Self | None: ...
    @property
    def payload(self) -> str: ...
    @property
    def template(self) -> str: ...
    def to_string(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> Self: ...
    def __deepcopy__(self, memo: object) -> Self: ...

class TraceReceiveVsync:
    thread_name: str
    thread_tid: int
    thread_tgid: int | None
    cpu: int
    flags: str
    timestamp: float
    event_name: str
    trace_mark_tgid: int
    frame_number: int
    def __init__(
        self,
        thread_name: str,
        thread_tid: int,
        thread_tgid: int | None,
        cpu: int,
        flags: str,
        timestamp: float,
        event_name: str,
        trace_mark_tgid: int,
        frame_number: int,
    ) -> None: ...
    @staticmethod
    def can_be_parsed(line: str) -> bool: ...
    @staticmethod
    def parse(line: str) -> Self | None: ...
    @property
    def payload(self) -> str: ...
    @property
    def template(self) -> str: ...
    def to_string(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> Self: ...
    def __deepcopy__(self, memo: object) -> Self: ...

class TraceSchedProcessExit:
    thread_name: str
    thread_tid: int
    thread_tgid: int | None
    cpu: int
    flags: str
    timestamp: float
    event_name: str
    comm: str
    pid: int
    prio: int
    group_dead: bool
    def __init__(
        self,
        thread_name: str,
        thread_tid: int,
        thread_tgid: int | None,
        cpu: int,
        flags: str,
        timestamp: float,
        event_name: str,
        comm: str,
        pid: int,
        prio: int,
        group_dead: bool,
    ) -> None: ...
    @staticmethod
    def can_be_parsed(line: str) -> bool: ...
    @staticmethod
    def parse(line: str) -> Self | None: ...
    @property
    def payload(self) -> str: ...
    @property
    def template(self) -> str: ...
    def to_string(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> Self: ...
    def __deepcopy__(self, memo: object) -> Self: ...

class TraceSchedSwitch:
    thread_name: str
    thread_tid: int
    thread_tgid: int | None
    cpu: int
    flags: str
    timestamp: float
    event_name: str
    prev_comm: str
    prev_pid: int
    prev_prio: int
    prev_state: str
    next_comm: str
    next_pid: int
    next_prio: int
    def __init__(
        self,
        thread_name: str,
        thread_tid: int,
        thread_tgid: int | None,
        cpu: int,
        flags: str,
        timestamp: float,
        event_name: str,
        prev_comm: str,
        prev_pid: int,
        prev_prio: int,
        prev_state: str,
        next_comm: str,
        next_pid: int,
        next_prio: int,
    ) -> None: ...
    @staticmethod
    def can_be_parsed(line: str) -> bool: ...
    @staticmethod
    def parse(line: str) -> Self | None: ...
    @property
    def payload(self) -> str: ...
    @property
    def template(self) -> str: ...
    def to_string(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> Self: ...
    def __deepcopy__(self, memo: object) -> Self: ...

class TraceSchedWakeup:
    thread_name: str
    thread_tid: int
    thread_tgid: int | None
    cpu: int
    flags: str
    timestamp: float
    event_name: str
    comm: str
    pid: int
    prio: int
    target_cpu: int
    success: bool | None
    reason: int | None
    def __init__(
        self,
        thread_name: str,
        thread_tid: int,
        thread_tgid: int | None,
        cpu: int,
        flags: str,
        timestamp: float,
        event_name: str,
        comm: str,
        pid: int,
        prio: int,
        target_cpu: int,
        success: bool | None,
        reason: int | None,
    ) -> None: ...
    @staticmethod
    def can_be_parsed(line: str) -> bool: ...
    @staticmethod
    def parse(line: str) -> Self | None: ...
    @property
    def payload(self) -> str: ...
    @property
    def template(self) -> str: ...
    def to_string(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> Self: ...
    def __deepcopy__(self, memo: object) -> Self: ...

class TraceSchedWakeupNew:
    thread_name: str
    thread_tid: int
    thread_tgid: int | None
    cpu: int
    flags: str
    timestamp: float
    event_name: str
    comm: str
    pid: int
    prio: int
    target_cpu: int
    def __init__(
        self,
        thread_name: str,
        thread_tid: int,
        thread_tgid: int | None,
        cpu: int,
        flags: str,
        timestamp: float,
        event_name: str,
        comm: str,
        pid: int,
        prio: int,
        target_cpu: int,
    ) -> None: ...
    @staticmethod
    def can_be_parsed(line: str) -> bool: ...
    @staticmethod
    def parse(line: str) -> Self | None: ...
    @property
    def payload(self) -> str: ...
    @property
    def template(self) -> str: ...
    def to_string(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> Self: ...
    def __deepcopy__(self, memo: object) -> Self: ...

class TracingMark:
    thread_name: str
    thread_tid: int
    thread_tgid: int | None
    cpu: int
    flags: str
    timestamp: float
    event_name: str
    def __init__(
        self,
        thread_name: str,
        thread_tid: int,
        thread_tgid: int | None,
        cpu: int,
        flags: str,
        timestamp: float,
        event_name: str,
    ) -> None: ...
    @staticmethod
    def can_be_parsed(line: str) -> bool: ...
    @staticmethod
    def parse(line: str) -> Self | None: ...
    @property
    def payload(self) -> str: ...
    @property
    def template(self) -> str: ...
    def to_string(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __str__(self) -> str: ...
    def __copy__(self) -> Self: ...
    def __deepcopy__(self, memo: object) -> Self: ...

def parse_trace(line: str) -> Trace | None: ...
def parse_trace_file(path: str, filter_event: str | None = None) -> list[Trace]: ...
def version() -> str: ...
