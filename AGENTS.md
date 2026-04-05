# trace_parser Agent Notes

Dense cheat sheet for fast onboarding: see **`AGENT_QUICKSTART.md`** (Cursor loads hard rules via `.cursor/rules/trace-parser-agent.mdc`).

## Project Goal

`trace_parser` is a `Rust + PyO3` parser for large text `ftrace` / `tracefs` logs.

Primary goals:

- parse the common trace line shape in Rust
- keep data in Rust until requested
- support typed event classes on top of a generic `Trace`
- support event-specific payload formats
- support semantic round-trip via `to_string()`
- allow service groups which are accepted during parse but omitted or normalized on output

## Tech Stack

- **Rust:** 1.94 (minimum 1.80 for `LazyLock`)
- **Edition:** 2024
- **PyO3:** 0.28 (with `Bound` API)
- **Python:** 3.10+
- **regex:** 1.12
- **memchr:** 2.7 (SIMD substring search)
- **lexical-core:** 1.0 (SIMD number parsing)

## SIMD Optimizations

The project uses SIMD instructions for faster parsing:

**memchr::memmem** — SIMD substring search:
```rust
use memchr::memmem;
let pos = memmem::find(line.as_bytes(), b": ")?;
```

**lexical-core::parse** — SIMD number parsing:
```rust
use lexical_core::parse;
let tid: u32 = parse(captures.name("tid")?.as_str().as_bytes()).ok()?;
let timestamp: f64 = parse(captures.name("timestamp")?.as_str().as_bytes()).ok()?;
```

**Used in:**
- `extract_event_name()` — memchr for finding `": "`
- `BaseTraceParts::parse()` — lexical-core for tid, tgid, cpu, timestamp
- `cap_parse()` — generic parser via `FromLexical`

Expected speedup: ~30-50% faster per-line parsing.

## PyO3 0.28 Notes

PyO3 0.28 uses the new `Bound` API:

- `IntoPy<T>` → `IntoPyObject<'py>`
- `PyObject` → `Py<PyAny>`
- `into_py(py)` → `into_pyobject(py)?.into_any().unbind()`
- `once_cell::sync::Lazy` → `std::sync::LazyLock`

Helper function for conversion:

```rust
fn parse_and_wrap<'py, T>(
    py: Python<'py>,
    line: &str,
    parser: fn(&str) -> Option<T>,
) -> Option<Py<PyAny>>
where
    T: IntoPyObject<'py>,
{
    parser(line)
        .and_then(|e| e.into_pyobject(py).ok())
        .map(|bound| bound.into_any().unbind())
}
```

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
- `uv pip install pre-commit`
- `uv run pre-commit install --hook-type pre-commit --hook-type pre-push --hook-type commit-msg`
- `maturin develop`
- `pytest tests/python`

Python packaging baseline:

- minimum supported Python version is `3.10`
- `pyo3` uses `abi3-py310`
- the native module is built as a package-local extension for `trace_parser._native`

Editable build artifacts:

- `maturin develop` places native artifacts such as `_native.abi3.so` under `trace_parser/`
- macOS may also emit `*.dSYM/` alongside that extension
- these are expected local build artifacts
- they must be ignored by git
- do not manually copy or symlink native libraries into `trace_parser/`

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

This rule is enforced via:

- a PR-only GitHub Actions check (`.github/workflows/commitlint.yml`)
- local `pre-commit` hooks (commit-msg stage, Conventional Commits check)

## Base Trace Format

Current parser targets trace lines shaped like:

```text
TASK-TID (TGID) [CPU] FLAGS TIMESTAMP: event_name: payload
```

Base fields (Rust / Python use `thread_tid` and `thread_tgid`; trace line still shows kernel `TASK-TID` and `(TGID)`):

- `thread_name`
- `thread_tid`
- `thread_tgid`
- `cpu`
- `flags`
- `timestamp`
- `event_name`
- `payload_raw` (only on generic `Trace` / `TracingMark`; typed template events use rendered `payload`)

## Architecture

### Base class

- `src/trace.rs`
- class: `Trace`
- generic parse for the common trace line

### Typed classes

Current typed classes:

- `Trace` (generic fallback, hand-written)
- `TraceSchedSwitch` (macro-generated)
- `TraceSchedWakeup` (macro-generated)
- `TraceSchedWakeupNew` (macro-generated)
- `TraceSchedProcessExit` (macro-generated)
- `TraceExit` (exit1, exit2, macro-generated)
- `TraceCpuFrequency` (macro-generated)
- `TraceDevFrequency` (macro-generated)
- `TracingMark` (macro-generated)
- `TraceMarkBegin` (macro-generated)
- `TraceMarkEnd` (macro-generated)
- `TraceReceiveVsync` (macro-generated)

### Flat typed events + proc-macro authoring

Typed event structs **repeat base trace fields** on the type (flat layout). Python accesses them directly (`event.timestamp`, `event.comm`, …).

Boilerplate for `EventType`, `FastMatch`, `TemplateEvent`, parser registration, and (optionally) `#[pymethods]` is generated by **`trace_parser_macros`**.

Recommended authoring style:

- `#[trace_event_class]` for normal events
- `#[tracing_mark_event_class]` for `tracing_mark_write` subtypes

The wrappers automatically add:

- `#[pyo3::pyclass(skip_from_py_object)]` by default (can be overridden via wrapper args, e.g. `from_py_object`)
- `#[derive(Clone, Debug, PartialEq)]`
- `#[derive(TraceEvent)]` or `#[derive(TracingMarkEvent)]`

Core attributes remain:

- `#[trace_event(...)]`, `#[define_template(...)]`, `#[trace_markers(...)]`, `#[field(...)]`

Field exposure rules for Python are defined via `#[field(...)]`:

- `#[field]` → getter + setter
- `#[field(readonly)]` → getter only
- `#[field(private)]` → no Python property

Use `#[trace_event(..., generate_pymethods = false)]` when the type needs a hand-written `#[pymethods]` block (custom `new`, extra methods, or tracing-mark helpers like `payload_to_string`).

**All typed events are now macro-generated.** The only hand-written event is `Trace` (generic fallback, not `TemplateEvent`).

For `TraceMarkBegin` / `TraceMarkEnd`, use `#[trace_event(..., register_tracing_mark = false)]` — the factory calls those parsers explicitly after the inventory pass, so they must not register into the tracing-mark inventory.

### File layout convention

- Put the `#[pyclass]` and derives first; then hand-written `#[pymethods]` if needed.
- Tests remain at the bottom.

Optional future work: PyO3 `extends` for shared base fields — [INHERITANCE_PLAN.md](INHERITANCE_PLAN.md).

### Fast-match heuristics

- `FastMatch::quick_check` uses `extract_event_name()` (SIMD via memchr) for event name extraction
- `FastMatch::PAYLOAD_MARKERS` — automatic SIMD payload checking with `memmem::find`
- `FastMatch::payload_quick_check()` — custom complex logic (override when markers aren't enough)
- `contains_all(line, [...])` and `contains_any(line, [...])` helpers exist for future multi-format heuristics
- `TraceDevFrequency` uses `#[fast_match(contains_any = ["clk=ddr_devfreq", "clk=l3c_devfreq"])]` so `payload_quick_check` requires one of those substrings
- `TraceReceiveVsync` uses `PAYLOAD_MARKERS = &[b"B|", b"ReceiveVsync"]`
- `TraceMarkBegin` uses `PAYLOAD_MARKERS = &[b"B|"]`
- `TraceMarkEnd` uses `PAYLOAD_MARKERS = &[b"E|"]`
- The heavy regex work is now gated by cheap fast checks, and `parse_trace()` routes a line to a single parser after these heuristics pass
- `benches/can_be_parsed.rs` captures the cost of each check path; rerun it whenever you touch the heuristic to judge regression risk
- Current design intent:
  - for ordinary non-`tracing_mark` events, `event_name` is usually enough for `quick_check`
  - payload-specific `PAYLOAD_MARKERS` should be used for simple marker checks (SIMD optimized)
  - `payload_quick_check()` should be used sparingly for complex logic that markers can't handle
  - `tracing_mark` subtypes use a separate registry with explicit parsing order
- `contains_all(...)` may be unused at times, but keep it because it is intended for future multi-format event matching

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

Optional payload fields are inferred from Rust type: use `Option<T>` directly (no `#[field(optional)]` flag).

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

### Parsing order (tracing_mark_registry.rs)

1. Registered specific subtypes (ReceiveVsync, RequestVsync, SubmitVsync, etc.)
2. TraceMarkBegin (hardcoded)
3. TraceMarkEnd (hardcoded)
4. TracingMark (fallback)

### FastMatch for tracing_mark

- `TraceReceiveVsync`: `PAYLOAD_MARKERS = &[b"B|", b"ReceiveVsync"]`
- `TraceMarkBegin`: `PAYLOAD_MARKERS = &[b"B|"]`
- `TraceMarkEnd`: `PAYLOAD_MARKERS = &[b"E|"]`
- `TracingMark`: default (no markers, accepts any payload)

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

Current performance intent:

- `parse_trace(...)` should rely on cheap quick checks before invoking regex-heavy parsing.
- `can_be_parsed()` for typed events is intended as a cheap heuristic, not a full parse guarantee.
- `parse()` should call the cheap `can_be_parsed()` first and only then do the full regex/template parse.

## Python-facing artifacts

Python package now lives under:

- `trace_parser/`

Native extension module name:

- `trace_parser._native`

Supported public import styles must both work:

- `from trace_parser import TraceReceiveVsync`
- `from trace_parser.tracing_mark.receive_vsync import TraceReceiveVsync`

When adding or changing public event classes, keep Python-facing files in sync.

Required updates:

- update runtime package files under `trace_parser/`
- update package-aligned `.pyi` files under `trace_parser/`
- update or add Python examples in `examples/`
- update or add Python smoke tests in `tests/python/`

This is not optional.

**Important:** When changing the public API (adding/removing fields, properties, methods, changing class structure), always verify that type stubs (`.pyi` files) are updated to match the new API. Stubs must be kept in sync with the Rust implementation.

For every new public typed event:

1. export the class from the Rust module and module entrypoint
2. export it from `trace_parser/__init__.py` if it is part of the flat public API
3. add or update the matching `.pyi` file under `trace_parser/`
4. update `trace_parser/__init__.pyi`
5. add at least one Python example using the class directly or through `parse_trace(...)`
6. add at least one Python smoke test that verifies the string parses successfully

Typing quality matters:

- package-local `.pyi` files under `trace_parser/` are the current source of truth
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

- `trace_parser/`

## Current priorities

1. ✅ Migrate all typed events onto `TraceEvent` / `TracingMarkEvent` — COMPLETED
2. Add more kernel event families (sched_migrate, sched_waking, etc.)
3. Keep event modules small and avoid unnecessary internal helper structs
4. E2E integration tests with real trace lines (see [macros/AGENTS.md](macros/AGENTS.md))

## Proc-macro status

**Done:**
- `macros/` crate with `#[derive(TraceEvent)]`, `#[derive(TracingMarkEvent)]`, and `#[derive(TraceEnum)]`
- Wrapper attribute macros: `#[trace_event_class]`, `#[tracing_mark_event_class]`
- Generates: `EventType`, `FastMatch`, `TemplateEvent`, optional `#[pymethods]` block (`generate_pymethods = false` supported)
- **Type inference** — `#[field]` without `ty`, inferred from Rust type
- **Optional inference** — `Option<T>` fields are treated as optional without `#[field(optional)]`
- **Custom regex** — `#[field(regex = r"...")]`
- **Format rendering** — `#[field(format = "{:03}")]` for integer fields (e.g. `target_cpu`)
- **`#[fast_match(contains_any = [...])]`** → `FastMatch::payload_quick_check` via `contains_any`
- **Multi-template `detect_format`** — SIMD detection via `detect = [...]` (e.g. `reason=`)
- **`render_payload` uses `format_id`** (multi-format round-trip)
- **`skip_registration`** for Begin/End (explicit registration)
- **TraceEnum derive** — `#[derive(TraceEnum)]`

**In use in `src/`:** ALL typed events — `TraceSchedSwitch`, `TraceSchedWakeup`, `TraceSchedWakeupNew`, `TraceSchedProcessExit`, `TraceExit`, `TraceCpuFrequency`, `TraceDevFrequency`, `TracingMark`, `TraceMarkBegin`, `TraceMarkEnd`, `TraceReceiveVsync` (see source). **Only hand-written:** `Trace` (generic fallback)

**Planned:**
- PyO3 `extends` (see INHERITANCE_PLAN.md)
- E2E integration tests with real trace lines (see [macros/AGENTS.md](macros/AGENTS.md))

## Deferred design issue

Typed tracing-mark special cases (`TraceReceiveVsync`) combine begin-marker fields with custom inner payload parsing on one flat struct. Cleaning that up may follow PyO3 `extends` work in [INHERITANCE_PLAN.md](INHERITANCE_PLAN.md).

## Validation

Use:

```bash
cargo test -q
```

Run Clippy (required in CI):

```bash
cargo clippy --all-targets -- -D warnings
```

Also use when touching Python/package behavior:

```bash
source .venv/bin/activate
maturin develop
pytest -q tests/python
```

And for fast-match changes:

```bash
cargo bench --bench can_be_parsed --quiet
```

Recent benchmark reference points for `sched_switch` on this machine:

- positive case:
  - `Trace::can_be_parsed` about `319 ns/op`
  - `TraceSchedSwitch::can_be_parsed` about `116 ns/op`
  - `TraceSchedSwitch::parse().is_some()` about `9.7 us/op`
- negative case:
  - `Trace::can_be_parsed` about `194 ns/op`
  - `TraceSchedSwitch::can_be_parsed` about `111 ns/op`
  - `TraceSchedSwitch::parse().is_some()` about `147 ns/op`

## Codacy Coverage

To enable coverage reporting on Codacy:

1. Go to https://app.codacy.com and add the repository
2. Get the Project Token from Settings → Coverage
3. Add GitHub Secret `CODACY_PROJECT_TOKEN` with the token value

Coverage is automatically uploaded from CI workflow.

Treat these only as rough regression anchors, not hard targets.
