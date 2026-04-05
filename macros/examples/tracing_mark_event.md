# Пример: Tracing Mark событие через wrapper macro

Использование `#[tracing_mark_event_class]` для подтипов `tracing_mark_write`.

```rust
use trace_parser_macros::tracing_mark_event_class;

#[tracing_mark_event_class]
#[trace_event(name = "tracing_mark_write", begin, skip_registration)]
#[define_template("{message}")]
struct TraceMarkBegin {
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
    pub trace_mark_tgid: u32,
    #[field]
    pub message: String,
}

#[tracing_mark_event_class(from_py_object)]
#[trace_event(name = "tracing_mark_write", end, skip_registration)]
#[define_template("{message}")]
struct TraceMarkEnd {
    #[field(private)]
    format_id: u8,
    #[field]
    pub thread_name: String,
    #[field(readonly)]
    pub event_name: String,
    #[field]
    pub trace_mark_tgid: u32,
    #[field]
    pub message: String,
}
```

## pyclass options

По умолчанию wrapper добавляет `#[pyo3::pyclass(skip_from_py_object)]`.

Можно передать аргументы в wrapper:

- `#[tracing_mark_event_class(from_py_object)]` — включить обратное поведение
- `#[tracing_mark_event_class(module = "trace_parser._native")]` — дополнительные параметры
- если явно указан `from_py_object` или `skip_from_py_object`, wrapper не добавляет второй конфликтующий флаг
