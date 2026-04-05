# trace_parser — agent quick reference

Dense onboarding for coding agents. **Canonical policies and edge cases:** `AGENTS.md`. **Macro crate:** `macros/AGENTS.md`. **PyO3 `extends` idea (not done):** `INHERITANCE_PLAN.md`.

## What this is

Rust + PyO3 parser for large **ftrace / tracefs** text logs. Parse in Rust; expose Python types on demand. **Edition 2024**, **PyO3 0.28** (`Bound` / `IntoPyObject`), **Python 3.10+**, **uv** + `.venv`.

## Line shape

```text
TASK-TID (TGID) [CPU] FLAGS TIMESTAMP: event_name: payload
```

Base fields live on `Trace` (`thread_name`, `thread_tid`, `thread_tgid`, `cpu`, `flags`, `timestamp`, `event_name`, `payload_raw` — exact names per `AGENTS.md` / code).

## Typed events (public surface)

`Trace`, `TraceSchedSwitch`, `TraceSchedWakeup`, `TraceSchedWakeupNew`, `TraceSchedProcessExit`, `TraceExit`, `TraceCpuFrequency`, `TraceDevFrequency`, `TracingMark`, `TraceMarkBegin`, `TraceMarkEnd`, `TraceReceiveVsync`.

Factory: `parse_trace(line)`. Bulk: `parse_trace_file(...)` (see README).

## Repo map

| Area | Location |
|------|----------|
| Core + dispatch | `src/lib.rs`, `src/trace.rs`, `src/registry.rs` |
| Event families | `src/sched_*.rs`, `src/frequency.rs`, `src/trace_exit.rs`, `src/tracing_mark/` |
| Payload templates | `src/payload_template.rs` |
| Format registry | `src/format_registry.rs` |
| Proc-macros | `macros/` (`trace_event_class`, `tracing_mark_event_class`, `TraceEvent`, `TracingMarkEvent`, `TraceEnum`) |
| Python package | `trace_parser/` (`__init__.py`, `*.pyi`, `_native.pyi`) |
| Python tests | `tests/python/` |

## Architecture rules (do not violate)

1. **Flat typed events:** typed events repeat base trace fields directly on the event type (`event.thread_tid`, `event.timestamp`, ...). Keep this flat layout consistent for Rust and Python.
2. **Fast path first:** `FastMatch::quick_check` + `PAYLOAD_MARKERS` / `payload_quick_check`; full regex/template only after cheap checks. Touch heuristics → consider `cargo bench --bench can_be_parsed`.
3. **Templates:** simple payloads → `PayloadTemplate` + `FieldSpec`; service tokens `{ws}`, `{?ws}`, `{ignore:…}`, `{?ignore:…}`.
4. **`tracing_mark_write`:** registry order in `tracing_mark_registry.rs` — specific subtypes → `TraceMarkBegin` → `TraceMarkEnd` → `TracingMark` fallback.
5. **`TraceDevFrequency`:** only `clock_set_rate` with allowed `clk` via `FieldSpec::choice` (list in `AGENTS.md` / code).
6. **Field exposure comes from `#[field(...)]`:** `#[field]` => getter+setter, `#[field(readonly)]` => getter only, `#[field(private)]` => not exposed to Python.
7. **Optional fields:** use `Option<T>`; do not use `#[field(optional)]`.

## SIMD / parsing habits

- `memchr` / `memmem` for finding `": "` and payload markers.
- `lexical-core` for numeric fields where the codebase already does.

## PyO3 0.28 (reminder)

Prefer `into_pyobject(py)?.into_any().unbind()` patterns; `LazyLock` not `once_cell::Lazy` where applicable.

## Python workflow

```bash
uv venv .venv -p 3.10
source .venv/bin/activate
uv pip install maturin pytest
uv pip install pre-commit
uv run pre-commit install --hook-type pre-commit --hook-type pre-push --hook-type commit-msg
maturin develop
pytest -q tests/python
```

Do not commit native artifacts under `trace_parser/` (gitignored).

## Validation (run what you touch)

| Change | Command |
|--------|---------|
| Rust | `cargo test -q` |
| CI parity | `cargo clippy --all-targets -- -D warnings` |
| Python / bindings | `maturin develop` + `pytest -q tests/python` |
| Fast-match | `cargo bench --bench can_be_parsed --quiet` |

## Changing the public API (checklist)

1. Rust export + `parse_trace` typing if needed.
2. `trace_parser/__init__.py` + matching `__init__.pyi`.
3. Submodule `.pyi` files — **stubs are source of truth** for typing.
4. Example under `examples/` and smoke test under `tests/python/`.

## Git

**Conventional Commits in English** only (enforced in CI).

## Known direction / drift

- **QWEN.md** is Russian context for another tool; may not match current code — prefer `AGENTS.md` + source.
- **Macro migration:** all typed events are already macro-generated; only `Trace` remains hand-written fallback (`macros/AGENTS.md`).
- **Python package** lives in `trace_parser/` (not `python/trace_parser/`).
