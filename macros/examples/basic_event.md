# Пример: Базовое событие через wrapper macro

Использование `#[trace_event_class]` для обычного события без ручных `#[pyclass]`, `#[derive(...)]` и `#[pyo3(get, set)]`.

```rust
use trace_parser_macros::trace_event_class;

#[trace_event_class]
#[trace_event(name = "sched_switch")]
#[define_template(
    "prev_comm={prev_comm} prev_pid={prev_pid} prev_prio={prev_prio} prev_state={prev_state} ==> next_comm={next_comm} next_pid={next_pid} next_prio={next_prio}"
)]
struct TraceSchedSwitch {
    #[field(private)]
    format_id: u8,

    #[field]
    pub thread_name: String,
    #[field]
    pub thread_tid: u32,
    #[field]
    pub thread_tgid: u32,
    #[field]
    pub cpu: u32,
    #[field]
    pub flags: String,
    #[field]
    pub timestamp: f64,

    #[field(readonly)]
    pub event_name: String,

    #[field]
    pub prev_comm: String,
    #[field]
    pub prev_pid: u32,
    #[field]
    pub prev_prio: i32,
    #[field]
    pub prev_state: String,
    #[field]
    pub next_comm: String,
    #[field]
    pub next_pid: u32,
    #[field]
    pub next_prio: i32,
}
```

## Python access rules

- `#[field]` → getter + setter
- `#[field(readonly)]` → getter only
- `#[field(private)]` → не экспортируется в Python

## Что генерируется

- `#[pyo3::pyclass(skip_from_py_object)]` по умолчанию
- `#[derive(Clone, Debug, PartialEq)]`
- `#[derive(TraceEvent)]`
- `impl EventType`, `impl FastMatch`, `impl TemplateEvent`, `#[pymethods]`, parser registration
