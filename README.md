# trace_parser

![Python Versions](https://img.shields.io/badge/Python-3.10%20%7C%203.11%20%7C%203.12%20%7C%203.13%20%7C%203.14-blue?logo=python&logoColor=white)

[![CI](https://github.com/amishenev/trace_parser/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/amishenev/trace_parser/actions/workflows/ci.yml)
[![Release](https://github.com/amishenev/trace_parser/actions/workflows/release.yml/badge.svg)](https://github.com/amishenev/trace_parser/actions/workflows/release.yml)
[![Codacy Grade](https://app.codacy.com/project/badge/Grade/690079fe36a24bc493148491caf7e16c)](https://app.codacy.com/gh/amishenev/trace_parser/dashboard?utm_source=gh&utm_medium=referral&utm_content=&utm_campaign=Badge_grade)
[![Codacy Coverage](https://app.codacy.com/project/badge/Coverage/690079fe36a24bc493148491caf7e16c)](https://app.codacy.com/gh/amishenev/trace_parser/dashboard?utm_source=gh&utm_medium=referral&utm_content=&utm_campaign=Badge_coverage)

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

| Event | Description |
|-------|-------------|
| `Trace` | Generic fallback for any trace line |
| `TraceSchedSwitch` | Scheduler context switch (`sched_switch`) |
| `TraceSchedWakeup` | Process wakeup (`sched_wakeup`) |
| `TraceSchedWakeupNew` | New process wakeup (`sched_wakeup_new`) |
| `TraceSchedProcessExit` | Process exit (`sched_process_exit`) |
| `TraceExit` | Kernel exit events (`exit1`, `exit2`) |
| `TraceCpuFrequency` | CPU frequency change (`cpu_frequency`) |
| `TraceDevFrequency` | Device frequency change (`clock_set_rate`) |
| `TracingMark` | Generic tracing mark (any payload) |
| `TraceMarkBegin` | Begin mark (`B|tgid|message`) |
| `TraceMarkEnd` | End mark (`E|tgid|message`) |
| `TraceReceiveVsync` | ReceiveVsync begin mark |

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
- Rust toolchain (1.83+, edition 2024)
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

Run Clippy:

```bash
cargo clippy --all-targets -- -D warnings
```

## Example

```python
from trace_parser import (
    TraceSchedSwitch,
    TraceSchedWakeup,
    TraceDevFrequency,
    TraceReceiveVsync,
    parse_trace,
)

# Factory parsing — auto-detects event type
line = (
    "bash-1977 (12) [000] .... 12345.678901: sched_switch: "
    "prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> "
    "next_comm=worker next_pid=123 next_prio=120"
)
event = parse_trace(line)
print(type(event).__name__)  # TraceSchedSwitch

# Direct parsing with typed class
switch = TraceSchedSwitch.parse(line)
assert switch is not None
print(switch.timestamp)
print(switch.prev_comm, switch.next_comm)

# Multi-format events (sched_wakeup with optional reason)
wakeup_line = (
    "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: "
    "comm=bash pid=1977 prio=120 target_cpu=000 reason=3"
)
wakeup = TraceSchedWakeup.parse(wakeup_line)
assert wakeup is not None
print(wakeup.reason)  # 3

# Device frequency (fast-match via SIMD)
freq_line = (
    "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: "
    "clk=ddr_devfreq state=933000000 cpu_id=0"
)
freq = TraceDevFrequency.parse(freq_line)
assert freq is not None
print(freq.clk, freq.state)

# Tracing marks (begin/end with TGID prefix)
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
src/                          # Rust source
  trace.rs                    # Base Trace class
  common.rs                   # Shared traits & utilities
  payload_template.rs         # Payload template system
  format_registry.rs          # Multi-format registry
  registry.rs                 # Event parser registry
  sched_switch.rs             # sched_switch (macro-generated)
  sched_wakeup.rs             # sched_wakeup / sched_wakeup_new (macro-generated)
  sched_process_exit.rs       # sched_process_exit
  frequency.rs                # cpu_frequency / clock_set_rate (macro-generated)
  trace_exit.rs               # exit1 / exit2
  tracing_mark/               # Tracing mark events
    base.rs                   # TracingMark, TraceMarkBegin, TraceMarkEnd
    receive_vsync.rs          # TraceReceiveVsync
  tracing_mark_registry.rs    # Tracing mark dispatch

trace_parser/                 # Python package
  __init__.py                 # Public API (re-exports from _native)
  __init__.pyi                # Type stubs
  _native.pyi                 # Native module stubs
  py.typed                    # PEP 561 marker

macros/                       # Proc-macro crate
  src/
    lib.rs                    # TraceEvent, TracingMarkEvent, TraceEnum
    attrs.rs                  # Attribute parsing
    generator.rs              # Trait generation
    pymethods.rs              # Python API generation
    enum_gen.rs               # TraceEnum generation
  examples/                   # Macro usage examples

tests/python/                 # Python smoke tests
examples/                     # Python usage examples
benches/                      # Rust benchmarks
```

## Development notes

- Python package typing lives in `trace_parser/*.pyi`
- `maturin develop` creates local native artifacts under `trace_parser/`
- those native build artifacts are ignored by git
- Python smoke tests live in `tests/python/`
- Minimum Rust version: 1.83 (edition 2024)
- PyO3 version: 0.28
- Proc-macros: `trace_parser_macros` crate (see `macros/` directory)

### Proc-macro system

Most typed events are generated via `#[derive(TraceEvent)]` or `#[derive(TracingMarkEvent)]` from the `trace_parser_macros` crate. The macro generates:

- `impl EventType` — event name and aliases
- `impl FastMatch` — SIMD-based quick checks
- `impl TemplateEvent` — payload parsing and rendering
- `#[pymethods]` — Python API (constructor, parse, to_string, etc.)
- Parser registration via `register_parser!`

See `macros/QWEN.md` for the full macro syntax reference.

## CI

GitHub Actions CI runs:

- `cargo test -q`
- `maturin develop`
- `pytest -q tests/python`
- on Python `3.10`, `3.11`, `3.12`, `3.13`, and `3.14`
- plus an allowed-to-fail check on `3.15-dev`

## Releases

GitHub Actions also provides a release workflow for GitHub Releases only.

When you push a tag like `v0.1.0`, it will:

- build wheels for Linux, macOS, and Windows
- build an sdist
- attach the artifacts to a GitHub Release

PyPI publishing is intentionally not configured yet.

Example:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Roadmap

- add more typed trace events (sched_migrate, sched_waking, etc.)
- migrate remaining hand-written events onto proc-macros
- PyO3 `extends` for shared base fields (see INHERITANCE_PLAN.md)
- E2E integration tests with real trace files
- stabilize the typed-event authoring pattern
- expand Python smoke coverage

## Performance

For bulk file parsing, use `parse_trace_file()` which is faster than line-by-line parsing:

```python
from trace_parser import parse_trace_file

# Parse entire file
events = parse_trace_file("trace.txt")

# Parse with event filtering (faster than filtering in Python)
events = parse_trace_file("trace.txt", filter_event="sched_switch")
```

This function reads and parses the file in Rust, avoiding the overhead of calling `parse_trace()` for each line from Python.

## Multiple format support

`trace_parser` supports multiple payload formats per event type through `FormatRegistry`.

Example with `sched_wakeup` which has two formats:

```python
# Default format
line1 = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000"
wakeup1 = TraceSchedWakeup.parse(line1)
print(wakeup1.format_kind)  # "orig"
print(wakeup1.reason)       # None

# Extended format with reason
line2 = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000 reason=3"
wakeup2 = TraceSchedWakeup.parse(line2)
print(wakeup2.format_kind)  # "with_reason"
print(wakeup2.reason)       # 3
```

Round-trip is preserved:

```python
rendered = wakeup2.to_string()
reparsed = TraceSchedWakeup.parse(rendered)
assert reparsed.format_kind == "with_reason"
assert reparsed.reason == 3
```
