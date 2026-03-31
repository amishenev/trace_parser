# Пример: Tracing Mark событие

Использование `#[derive(TracingMarkEvent)]` для подтипов `tracing_mark_write`.

```rust
use trace_parser_macros::TracingMarkEvent;

// Базовая метка начала
#[trace_event(name = "tracing_mark_write")]
#[trace_markers("B|")]
#[define_template("B|{trace_mark_tgid}|{message}")]
#[derive(TracingMarkEvent)]
struct TraceMarkBegin {
    #[field(ty = "u32")]
    trace_mark_tgid: u32,

    #[field(ty = "string")]
    message: String,
}

// Специфичная метка ReceiveVsync
#[trace_event(name = "tracing_mark_write")]
#[trace_markers("B|", "ReceiveVsync")]
#[define_template("{?ignore:extra_info}ReceiveVsync {frame_number}")]
#[derive(TracingMarkEvent)]
struct TraceReceiveVsync {
    #[field(ty = "u32")]
    frame_number: u32,
}
```

## Что генерируется

- `impl FastMatch` с `PAYLOAD_MARKERS` для быстрой проверки
- Регистрация через `register_tracing_mark_parser!`
- Парсинг через `tracing_mark_registry::parse_tracing_mark()`

## Порядок парсинга

1. Зарегистрированные специфичные события (ReceiveVsync)
2. TraceMarkBegin (захардкожено)
3. TraceMarkEnd (захардкожено)
4. TracingMark (fallback)

## Использование

```rust
// Begin
let begin = TraceMarkBegin::new(
    "any_thread".to_string(), 232, 10, 0, "....".to_string(), 12345.678900,
    "tracing_mark_write".to_string(),
    10, "some_custom_message".to_string(),
).unwrap();

// ReceiveVsync
let vsync = TraceReceiveVsync::new(
    "any_thread".to_string(), 232, 10, 0, "....".to_string(), 12345.678900,
    "tracing_mark_write".to_string(),
    10, "ReceiveVsync 42".to_string(),
    42,
).unwrap();
```
