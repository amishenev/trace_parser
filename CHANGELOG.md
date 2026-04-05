# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.3] - 2026-04-05

### Changed

- Dispatcher behavior is now strict for known events:
  - unknown `event_name` values still fall back to base `Trace` parsing
  - known events with unsupported payload format now raise `ValueError`
- Added Python smoke tests for:
  - unknown event fallback to `Trace`
  - `ValueError` on unsupported known-event format

## [0.3.2] - 2026-04-05

### Added

- Wrapper attribute macros for typed events:
  - `#[trace_event_class]`
  - `#[tracing_mark_event_class]`
- Wrapper `pyclass` option pass-through with sensible default:
  - defaults to `skip_from_py_object`
  - supports explicit `from_py_object` and other `pyclass(...)` options
- Additional tests for:
  - type-based optional inference from `Option<T>`
  - false-positive prevention for optional inference
  - `pyclass` wrapper option behavior (`skip_from_py_object` vs `from_py_object`)
  - Python field access rules (`private`, `readonly`, read-write)

### Changed

- Typed event modules migrated to wrapper macro style to reduce per-event boilerplate.
- Python field property generation is now driven by `#[field(...)]` metadata:
  - `#[field]` -> getter + setter
  - `#[field(readonly)]` -> getter only
  - `#[field(private)]` -> not exposed to Python
- Optional payload behavior is inferred from Rust field type `Option<T>`; `#[field(optional)]` is no longer needed.
- Macro examples updated to reflect wrapper macro usage and `Option<T>`-based optional inference.
- Agent/developer docs updated (`AGENTS.md`, `AGENT_QUICKSTART.md`, `QWEN.md`) for the new macro workflow.

### Migration Notes

- Public runtime Python API remains backward compatible.
- For new typed events, prefer wrapper macros (`trace_event_class` / `tracing_mark_event_class`) over manual `#[pyclass]` + derive boilerplate.

## [0.3.0] - 2026-04-04

### Added

- **Proc-macro system** (`trace_parser_macros` crate) — automatic code generation for typed trace events
  - `#[derive(TraceEvent)]` for standard events (e.g. `sched_switch`, `sched_wakeup`, `cpu_frequency`)
  - `#[derive(TracingMarkEvent)]` for `tracing_mark_write` subtypes (e.g. `TraceMarkBegin`, `TraceMarkEnd`, `TraceReceiveVsync`)
  - `#[derive(TraceEnum)]` for enum payload fields with `#[value("...")]` attributes
- **Field attribute features**:
  - Automatic type inference from Rust type (`String` → `string()`, `u32` → `u32()`, `bool` → `bool_int()`, etc.)
  - Custom regex patterns via `#[field(regex = r"...")]`
  - Choice constraints via `#[field(choice = ["a", "b"])]`
  - Custom rendering format via `#[field(format = "{:03}")]`
  - Optional fields via `#[field(optional)]` (`Option<T>`)
  - Read-only and private field flags
- **Multi-template support** — events can have multiple payload formats with automatic SIMD-based detection via `detect = ["..."]` markers
- **Fast-match optimization** — `#[fast_match(contains_any = [...])]` for payload quick checks using SIMD `memmem::find`
- **Inline extra_fields** — ignored fields (e.g. `{?ignore:extra_info}`) can define their regex inline via template attribute
- **Event alias support** — single event type can match multiple `event_name` values
- **CI improvements**:
  - Added macro crate tests to CI and coverage workflows (95 total tests: 37 main + 58 macros)
  - Release workflow now builds wheels for Python 3.10–3.14 across Linux, macOS, and Windows (15 wheel artifacts)
  - Separate Python 3.15-dev build job with `continue-on-error`

### Changed

- **All typed events migrated to proc-macros** — 80%+ boilerplate reduction in event source files:
  - `TraceSchedSwitch`
  - `TraceSchedWakeup` / `TraceSchedWakeupNew`
  - `TraceSchedProcessExit`
  - `TraceExit` (exit1, exit2)
  - `TraceCpuFrequency` / `TraceDevFrequency`
  - `TracingMark` / `TraceMarkBegin` / `TraceMarkEnd`
  - `TraceReceiveVsync`
- **Removed wrapper macros** — `register_parser!` and `register_tracing_mark_parser!` replaced with direct `inventory::submit!` generation
- **Consolidated `contains_any`** — inlined into macro-generated `FastMatch::payload_quick_check`, removed from `common.rs`
- **Updated documentation** — AGENTS.md, QWEN.md, macros/QWEN.md, macros/AGENTS.md, README.md all reflect new macro-based architecture

### Fixed

- Multi-template `render_payload` now correctly uses `format_id` for round-trip fidelity
- Optional fields passed directly in `new()` constructor instead of nested tuples
- Template name collisions resolved via per-struct unique static names
- Alias matching in `parse_event` via `matches_event_name` helper

### Removed

- Manual event implementations for all typed events except `Trace` (base fallback class)

### Migration Notes

- **For users**: No breaking changes to the public Python API. All existing code continues to work.
- **For contributors**: New typed events should use `#[derive(TraceEvent)]` or `#[derive(TracingMarkEvent)]` — see `macros/QWEN.md` for syntax reference.
- The only hand-written event is `Trace` (generic fallback, not a `TemplateEvent`).
