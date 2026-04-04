# Пример: Tracing Mark событие

Использование `#[derive(TracingMarkEvent)]` для подтипов `tracing_mark_write`.

```rust
use trace_parser_macros::TracingMarkEvent;

// Базовая метка начала — префикс B|{trace_mark_tgid}| добавляется автоматически
#[trace_event(name = "tracing_mark_write", begin, skip_registration, generate_pymethods = false)]
#[define_template("{message}")]
#[derive(TracingMarkEvent)]
struct TraceMarkBegin {
    #[field]
    trace_mark_tgid: u32,

    #[field]
    message: String,
}

// Базовая метка конца — префикс E|{trace_mark_tgid}| добавляется автоматически
#[trace_event(name = "tracing_mark_write", end, skip_registration, generate_pymethods = false)]
#[define_template("{message}")]
#[derive(TracingMarkEvent)]
struct TraceMarkEnd {
    #[field]
    trace_mark_tgid: u32,

    #[field]
    message: String,
}

// Специфичная метка ReceiveVsync — маркеры объединяются: ["B|", "ReceiveVsync"]
// extra_info — игнорируемое поле, не создаётся в структуре
#[trace_event(name = "tracing_mark_write", begin)]
#[trace_markers("ReceiveVsync")]
#[define_template("{?ignore:extra_info}ReceiveVsync {frame_number}", extra_info = r"\[[^\]]+\]")]
#[derive(TracingMarkEvent)]
struct TraceReceiveVsync {
    #[field]
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
