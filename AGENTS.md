# trace_parser Agent Notes

## Project Goal

`trace_parser` is a `Rust + PyO3` parser for large text `ftrace` / `tracefs` logs.

Primary goals:

- parse the common trace line shape in Rust
- keep data in Rust until requested
- support typed event classes on top of a generic `Trace`
- support event-specific payload formats
- support semantic round-trip via `to_string()`
- allow service groups which are accepted during parse but omitted or normalized on output

## Python workflow

Use `uv` for Python work in this repository.

Current rule:

- create and manage the local virtual environment as `.venv`
- create the environment with an explicit Python version
- activate the environment before installing or running Python tools
- install Python-side tools into the active environment via `uv`
- avoid plain `pip` for project work unless there is a concrete blocker

Current intended dev flow:

- `uv venv .venv -p 3.10`
- `source .venv/bin/activate`
- `uv pip install maturin pytest`
- `maturin develop`
- `pytest tests/python`

Python packaging baseline:

- minimum supported Python version is `3.10`
- `pyo3` uses `abi3-py310`
- the native module is built as a package-local extension for `trace_parser._native`

Editable build artifacts:

- `maturin develop` places native artifacts such as `_native.abi3.so` under `python/trace_parser/`
- macOS may also emit `*.dSYM/` alongside that extension
- these are expected local build artifacts
- they must be ignored by git
- do not manually copy or symlink native libraries into `python/trace_parser/`

## Commit messages

All future commit messages in this repository must:

- use Conventional Commits
- be written in English

Preferred examples:

- `feat: add sched_waking parser`
- `fix: separate pyo3 extension-module feature for CI`
- `docs: update README with release workflow`
- `ci: expand Python version matrix`

Do not use free-form commit messages in this repository anymore.

This rule is also enforced in GitHub Actions through a dedicated commitlint workflow.

## Base Trace Format

Current parser targets trace lines shaped like:

```text
TASK-TID (TGID) [CPU] FLAGS TIMESTAMP: event_name: payload
```

Base fields:

- `thread_name`
- `tid`
- `tgid`
- `cpu`
- `flags`
- `timestamp`
- `event_name`
- `payload_raw`

## Architecture

### Base class

- `src/trace.rs`
- class: `Trace`
- generic parse for the common trace line

### Typed classes

Current typed classes:

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

### Composition, not inheritance

Typed classes should use composition:

- `TraceSchedSwitch` contains `base: Trace`
- `TraceSchedWakeup` contains `base: Trace`
- `TraceSchedWakeupNew` contains `base: Trace`
- `TraceSchedProcessExit` contains `base: Trace`
- `TraceCpuFrequency` contains `base: Trace`
- `TraceDevFrequency` contains `base: Trace`
- `TracingMark` contains `base: Trace`
- `TraceMarkBegin` contains `mark: TracingMark`
- `TraceMarkEnd` contains `mark: TracingMark`
- `TraceReceiveVsync` contains `begin: TraceMarkBegin`

Do not duplicate all base `Trace` getters/setters into typed classes.
Access shared fields through the nested object.

Examples:

- `sched_switch.base.timestamp`
- `dev_frequency.state`
- `tracing_mark.base.payload_raw`
- `receive_vsync.begin.mark.base.thread_name`

Current tradeoff:

- this composition model is simple and stable in Rust
- but deep Python access like `vsync.begin.mark.base.timestamp` is too verbose

Current decision:

- do not add ad-hoc proxy getters/setters yet
- do not switch to inheritance
- revisit flattened Python access only after the typed-event authoring pattern is stable

Important constraint for future work:

- if flattened access like `vsync.timestamp` is added later, it must come from one shared mechanism
- do not add one-off convenience properties per class

## Payload templates

### File

- `src/payload_template.rs`

### Purpose

Simple payload formats should be described once and reused for:

- regex generation
- string formatting

### Current syntax

Template example:

```text
prev_comm={prev_comm} prev_pid={prev_pid} ==> next_comm={next_comm} next_pid={next_pid}
```

Field specs are declared separately:

```rust
&[
    ("prev_comm", FieldSpec::string()),
    ("prev_pid", FieldSpec::u32()),
    ("next_comm", FieldSpec::string()),
    ("next_pid", FieldSpec::u32()),
]
```

### Supported field specs

- `FieldSpec::string()`
- `FieldSpec::u32()`
- `FieldSpec::i32()`
- `FieldSpec::f64()`
- `FieldSpec::bool_int()`
- `FieldSpec::choice(&[...])`
- `FieldSpec::custom(...)`

Even if some are not used yet, keep them for future event families.

### Supported template values

- `TemplateValue::Str(...)`
- `TemplateValue::U32(...)`
- `TemplateValue::I32(...)`
- `TemplateValue::F64(...)`
- `TemplateValue::BoolInt(...)`

`BoolInt` means:

- in the text format the field is `0` / `1`
- in the typed Rust/Python model the field is `bool`
- in `to_string()` it must be rendered back as `0` / `1`

### Rule for payload helper structs

Do not keep separate `*Payload` structs for simple one-shot payload parsing.

Current preferred rule:

- if payload parsing is straightforward and used only once, parse directly from regex captures into the final typed event
- introduce a separate payload struct only when the payload logic becomes meaningfully complex

This is important for keeping the codebase small and obvious.

## Service groups

Service groups are accepted during parse but not stored as typed business fields.

Current built-in service tokens in payload templates:

- `{ws}`: matches `\s+`, renders as a single space
- `{?ws}`: matches `\s*`, renders as nothing
- `{ignore:name}`: matches a service group and omits it from output
- `{?ignore:name}`: same, but optional

Important:

- `sched_switch` should keep literal spaces for now
- service whitespace support exists for other events and for template tests

Current intended usage:

- prefer service tokens in templates over event-local regex hacks
- use `{?ignore:name}` for things like `[ExtraInfo]`

## tracing_mark rules

### Event

- all tracing mark events have `event_name == "tracing_mark_write"`

### Hierarchy intent

- `TracingMark`: accepts any payload for `tracing_mark_write`
- `TraceMarkBegin`: payload shape `B|trace_mark_tgid|payload`
- `TraceMarkEnd`: payload shape `E|trace_mark_tgid|payload`
- `TraceReceiveVsync`: specific begin mark example

### Important unfinished work

`tracing_mark` should follow the same payload-template philosophy as other simple events.

Specifically:

- `TraceMarkBegin` and `TraceMarkEnd` should stay template-driven
- `TraceReceiveVsync` should model `[ExtraInfo]` as an ignored service group
- future tracing_mark subtypes should prefer `PayloadTemplate` over custom regex where practical

Current status:

- `TracingMark`, `TraceMarkBegin`, `TraceMarkEnd`, and `TraceReceiveVsync` already use the payload-template approach
- `[ExtraInfo]` is modeled via `{?ignore:name}`
- keep following this style for new trace mark subtypes

## sched rules

### `sched_switch`

Current expected payload shape:

```text
prev_comm={prev_comm} prev_pid={prev_pid} prev_prio={prev_prio} prev_state={prev_state} ==> next_comm={next_comm} next_pid={next_pid} next_prio={next_prio}
```

### `sched_wakeup` and `sched_wakeup_new`

Current expected payload shape:

```text
comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}
```

`target_cpu` is rendered canonically as zero-padded 3 digits.

### `sched_process_exit`

Current expected payload shape:

```text
comm={comm} pid={pid} prio={prio} group_dead={group_dead}
```

`group_dead` must use `FieldSpec::bool_int()` and render back as `0` / `1`, not `true` / `false`.

## Frequency rules

### File

- `src/frequency.rs`

### Supported frequency-related classes

- `TraceCpuFrequency`
- `TraceDevFrequency`

### `TraceCpuFrequency`

Expected payload:

```text
state={state} cpu_id={cpu_id}
```

### `TraceDevFrequency`

This is the public typed event for selected `clock_set_rate` entries.

It should parse only when:

- `event_name == "clock_set_rate"`
- all required payload fields are present
- `clk` is in the allowed device list

Current allowed values:

- `ddr_devfreq`
- `l3c_devfreq`

This filter should be expressed through `FieldSpec::choice(...)`, not ad-hoc `matches!(...)`.

When the device list changes later, update the `choice(...)` list.

Internally, `frequency.rs` may still use helper parsing structures, but `TraceClockSetRate` is not part of the intended public Python API anymore.

## Factory parser

In `src/lib.rs`, keep a factory function exported through the package:

```python
trace_parser.parse_trace(line)
```

It should return the most specific known class first, then fall back to `Trace`, then `None`.

## Python-facing artifacts

Python package now lives under:

- `python/trace_parser/`

Native extension module name:

- `trace_parser._native`

Supported public import styles must both work:

- `from trace_parser import TraceReceiveVsync`
- `from trace_parser.tracing_mark.receive_vsync import TraceReceiveVsync`

When adding or changing public event classes, keep Python-facing files in sync.

Required updates:

- update runtime package files under `python/trace_parser/`
- update package-aligned `.pyi` files under `python/trace_parser/`
- update or add Python examples in `examples/`
- update or add Python smoke tests in `tests/python/`

This is not optional.

For every new public typed event:

1. export the class from the Rust module and module entrypoint
2. export it from `python/trace_parser/__init__.py` if it is part of the flat public API
3. add or update the matching `.pyi` file under `python/trace_parser/`
4. update `python/trace_parser/__init__.pyi`
5. add at least one Python example using the class directly or through `parse_trace(...)`
6. add at least one Python smoke test that verifies the string parses successfully

Typing quality matters:

- package-local `.pyi` files under `python/trace_parser/` are the current source of truth
- do not reintroduce a separate `stubs/` directory
- typing should be explicit and convenient
- include every public class
- include every public method
- include nested object fields exposed to Python
- keep `parse_trace(...)` return typing updated
- keep root-package typing and submodule typing both valid

## File layout

Keep code split by event family:

- `trace.rs`
- `sched_switch.rs`
- `sched_wakeup.rs`
- `sched_process_exit.rs`
- `frequency.rs`
- `tracing_mark/base.rs`
- `tracing_mark/receive_vsync.rs`
- future:
  - additional trace mark subtype files if needed

Tests should live near the code they verify.

Python smoke tests live in:

- `tests/python/`

Python package files live in:

- `python/trace_parser/`

## Current priorities

1. Keep composition for now and postpone flattened Python access until there is one shared mechanism
2. Add more kernel event families after that
3. Keep event modules small and avoid unnecessary internal helper structs

## Deferred design issue

Desired Python ergonomics:

- `vsync.timestamp`

Current internal access shape:

- `vsync.begin.mark.base.timestamp`

We intentionally defer this problem for now.

When revisiting, choose only one of:

1. Keep nested composition access as the public API
2. Add one shared proxy-field mechanism for all typed events

Do not implement class-by-class convenience accessors before that decision is made.

## Validation

Use:

```bash
cargo test -q
```
