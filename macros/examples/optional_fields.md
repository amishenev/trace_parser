# Пример: Опциональные поля и multi-template

Использование `#[field(optional)]` и `detect = [...]` для событий с несколькими форматами.

```rust
use pyo3::prelude::*;
use trace_parser_macros::TraceEvent;

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
#[derive(TraceEvent)]
#[trace_event(name = "sched_wakeup")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu} reason={reason}", detect = ["reason="])]
struct TraceSchedWakeup {
    #[field]
    format_id: u8,
    #[pyo3(get, set)]
    #[field]
    pub thread_name: String,
    #[pyo3(get, set)]
    #[field]
    pub thread_tid: u32,
    #[pyo3(get, set)]
    #[field]
    pub thread_tgid: u32,
    #[pyo3(get, set)]
    #[field]
    pub cpu: u32,
    #[pyo3(get, set)]
    #[field]
    pub flags: String,
    #[pyo3(get, set)]
    #[field]
    pub timestamp: f64,
    #[pyo3(get)]
    #[field]
    pub event_name: String,
    #[field]
    pub comm: String,

    #[field]
    pub pid: u32,

    #[field]
    pub prio: i32,

    #[field(format = "{:03}")]
    pub target_cpu: u32,

    #[field(optional)]
    pub reason: Option<u32>,  // ← опциональное поле
}
```

## Что генерируется

- `reason` имеет тип `Option<u32>` в Rust
- В Python поле может быть `None`
- Конструктор принимает `Option<u32>` для `reason`
- `detect_format` через SIMD `memmem::find` ищет `"reason="`

## Использование

```rust
// С reason
let event_with = TraceSchedWakeup::new(
    "kworker".to_string(), 123, 123, 0, "....".to_string(), 12345.679001,
    "sched_wakeup".to_string(),
    "bash".to_string(), 1977, 120, 0, Some(3),
).unwrap();

// Без reason
let event_without = TraceSchedWakeup::new(
    "kworker".to_string(), 123, 123, 0, "....".to_string(), 12345.679001,
    "sched_wakeup".to_string(),
    "bash".to_string(), 1977, 120, 0, None,
).unwrap();
```
