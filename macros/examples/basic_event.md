# Пример: Базовое событие

Использование `#[derive(TraceEvent)]` для простого события без опциональных полей.

```rust
use pyo3::prelude::*;
use trace_parser_macros::TraceEvent;

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
#[derive(TraceEvent)]
#[trace_event(name = "sched_switch")]
#[define_template(
    "prev_comm={prev_comm} prev_pid={prev_pid} prev_prio={prev_prio} prev_state={prev_state} ==> next_comm={next_comm} next_pid={next_pid} next_prio={next_prio}"
)]
struct TraceSchedSwitch {
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

## Что генерируется

- `impl EventType` с `EVENT_NAME = "sched_switch"`
- `impl FastMatch` с пустыми маркерами
- `impl TemplateEvent` с одним форматом
- `#[pymethods]` с конструктором, `can_be_parsed()`, `parse()`, `to_string()`, геттерами/сеттерами
- Регистрация через `inventory::submit!`

## Использование

```rust
// Через конструктор
let event = TraceSchedSwitch::new(
    "bash".to_string(), 1234, 1234, 0, "....".to_string(), 12345.678901,
    "sched_switch".to_string(),
    "bash".to_string(), 1977, 120, "S".to_string(),
    "worker".to_string(), 123, 120,
).unwrap();

println!("{}", event.to_string().unwrap());
// bash-1234 (1234) [000] .... 12345.678901: sched_switch: prev_comm=bash ...

// Через парсинг строки
let line = "bash-1977 (12) [000] .... 12345.678901: sched_switch: prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> next_comm=worker next_pid=123 next_prio=120";
let event = TraceSchedSwitch::parse(line).unwrap();
assert_eq!(event.prev_comm, "bash");
assert_eq!(event.next_pid, 123);
```
