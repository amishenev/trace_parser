# trace_parser

[![CI](https://github.com/amishenev/trace_parser/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/amishenev/trace_parser/actions/workflows/ci.yml)
[![Release](https://github.com/amishenev/trace_parser/actions/workflows/release.yml/badge.svg)](https://github.com/amishenev/trace_parser/actions/workflows/release.yml)

`trace_parser` is a `Rust + PyO3` library for parsing large text `ftrace` / `tracefs` logs.

It targets lines shaped like:

```text
TASK-TID (TGID) [CPU] FLAGS TIMESTAMP: event_name: payload
```

The current design keeps parsing in Rust and exposes Python classes only when requested.

## Goals

- parse common trace headers quickly in Rust
- keep typed event support on top of a generic `Trace`
- support event-specific payload formats
- support semantic round-trip via `from_string()` / `to_string()`
- keep simple event formats easy to extend through `PayloadTemplate`

## Current event support

- `Trace`
- `TraceSchedSwitch`
- `TraceSchedWakeup`
- `TraceSchedWakeupNew`
- `TraceSchedProcessExit`
- `TraceCpuFrequency`
- `TraceDevFrequency`
- `TracingMark`
- `TraceMarkBegin`
- `TraceMarkEnd`
- `TraceReceiveVsync`

## Python API

Both import styles are supported:

```python
from trace_parser import TraceReceiveVsync
from trace_parser.tracing_mark.receive_vsync import TraceReceiveVsync
```

There is also a factory parser:

```python
from trace_parser import parse_trace

event = parse_trace(line)
```

## Quick start

Requirements:

- Python `3.10+`
- Rust toolchain
- `uv`

Create the environment and install development tools:

```bash
uv venv .venv -p 3.10
source .venv/bin/activate
uv pip install maturin pytest
```

Build the extension into the Python package:

```bash
maturin develop
```

Run tests:

```bash
cargo test -q
pytest -q tests/python
```

## Installation for development

```bash
uv venv .venv -p 3.10
source .venv/bin/activate
uv pip install maturin pytest
maturin develop
```

After that, imports work directly from the local package:

```python
from trace_parser import Trace
from trace_parser.tracing_mark.receive_vsync import TraceReceiveVsync
```

## Example

```python
from trace_parser import TraceSchedSwitch, TraceDevFrequency, TraceReceiveVsync

switch_line = (
    "bash-1977 (12) [000] .... 12345.678901: sched_switch: "
    "prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> "
    "next_comm=worker next_pid=123 next_prio=120"
)

switch = TraceSchedSwitch.parse(switch_line)
assert switch is not None
print(switch.base.timestamp)
print(switch.prev_comm, switch.next_comm)

freq_line = (
    "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: "
    "clk=ddr_devfreq state=933000000 cpu_id=0"
)

freq = TraceDevFrequency.parse(freq_line)
assert freq is not None
print(freq.clk, freq.state)

vsync_line = (
    "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: "
    "B|10|[ExtraInfo]ReceiveVsync 42"
)

vsync = TraceReceiveVsync.parse(vsync_line)
assert vsync is not None
print(vsync.frame_number)
```

## Project layout

```text
src/
  trace.rs
  sched_switch.rs
  sched_wakeup.rs
  sched_process_exit.rs
  frequency.rs
  tracing_mark/
    base.rs
    receive_vsync.rs

python/trace_parser/
  __init__.py
  __init__.pyi
  trace.py
  frequency.py
  sched_switch.py
  sched_wakeup.py
  sched_process_exit.py
  tracing_mark/
    base.py
    receive_vsync.py
```

## Development notes

- Python package typing lives in `python/trace_parser/*.pyi`
- `maturin develop` creates local native artifacts under `python/trace_parser/`
- those native build artifacts are ignored by git
- Python smoke tests live in `tests/python/`

## CI

GitHub Actions CI runs:

- `cargo test -q`
- `maturin develop`
- `pytest -q tests/python`

## Releases

GitHub Actions also provides a release workflow.

When you push a tag like `v0.1.0`, it will:

- build wheels for Linux, macOS, and Windows
- build an sdist
- attach the artifacts to a GitHub Release

Example:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Roadmap

- add more typed trace events
- stabilize the typed-event authoring pattern
- improve Python-side ergonomics for common base fields
- expand Python smoke coverage and release artifacts
