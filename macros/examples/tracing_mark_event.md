# Пример: Tracing Mark событие

Использование `#[derive(TracingMarkEvent)]` для подтипов `tracing_mark_write`.

```rust
use pyo3::prelude::*;
use trace_parser_macros::TracingMarkEvent;

// Базовая метка начала — префикс B|{trace_mark_tgid}| добавляется автоматически
#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
#[derive(TracingMarkEvent)]
#[trace_event(name = "tracing_mark_write", begin, skip_registration)]
#[define_template("{message}")]
struct TraceMarkBegin {
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
    pub trace_mark_tgid: u32,
    #[field]
    pub message: String,
}

// Базовая метка конца — префикс E|{trace_mark_tgid}| добавляется автоматически
#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
#[derive(TracingMarkEvent)]
#[trace_event(name = "tracing_mark_write", end, skip_registration)]
#[define_template("{message}")]
struct TraceMarkEnd {
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
    pub trace_mark_tgid: u32,
    #[field]
    pub message: String,
}

// Специфичная метка ReceiveVsync — маркеры объединяются: ["B|", "ReceiveVsync"]
// extra_info — игнорируемое поле, не создаётся в структуре
#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
#[derive(TracingMarkEvent)]
#[trace_event(name = "tracing_mark_write", begin)]
#[trace_markers("ReceiveVsync")]
#[define_template("{?ignore:extra_info}ReceiveVsync {frame_number}", extra_info = r"\[[^\]]+\]")]
struct TraceReceiveVsync {
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
    pub trace_mark_tgid: u32,
    #[field]
    pub frame_number: u32,
}
```

## Что генерируется

- `impl FastMatch` с `PAYLOAD_MARKERS` для быстрой проверки
- Регистрация через `inventory::submit!` (кроме Begin/End с `skip_registration`)
- Парсинг через `tracing_mark_registry::parse_tracing_mark()`

## Порядок парсинга

1. Зарегистрированные специфичные события (ReceiveVsync)
2. TraceMarkBegin (захардкожено)
3. TraceMarkEnd (захардкожено)
4. TracingMark (fallback)

## Использование

```rust
// Begin
let begin = TraceMarkBegin::parse(
    "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|some_custom_message"
).unwrap();
assert_eq!(begin.trace_mark_tgid, 10);
assert_eq!(begin.message, "some_custom_message");

// ReceiveVsync
let vsync = TraceReceiveVsync::parse(
    "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|[ExtraInfo]ReceiveVsync 42"
).unwrap();
assert_eq!(vsync.frame_number, 42);
```
