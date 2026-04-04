# Пример: Кастомное имя поля

Использование `#[field(name = "...")]` для маппинга имён.

```rust
use pyo3::prelude::*;
use trace_parser_macros::TraceEvent;

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
#[derive(TraceEvent)]
#[trace_event(name = "cpu_frequency")]
#[define_template("state={state} cpu_id={cpu_id}")]
struct TraceCpuFrequency {
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
    #[field(name = "state")]
    pub current_state: u32,  // ← имя переменной != имя в payload

    #[field]
    pub cpu_id: u32,  // ← имя совпадает
}
```

## Что генерируется

- `impl TemplateEvent` с маппингом `current_state` → `state`
- В `parse_payload()` поле `state` из шаблона маппится на `current_state`
- В `render_payload()` поле `current_state` рендерится как `state`

## Использование

```rust
let event = TraceCpuFrequency::new(
    "swapper".to_string(), 0, 0, 0, "....".to_string(), 12345.678900,
    "cpu_frequency".to_string(),
    933000000, 0,
).unwrap();

// payload будет "state=933000000 cpu_id=0"
println!("{}", event.payload().unwrap());
```
