# Пример: Опциональные поля и multi-template

Использование `#[field(optional)]` и `detect = [...]` для событий с несколькими форматами.

```rust
use trace_parser_macros::TraceEvent;

#[trace_event(name = "sched_wakeup")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu} reason={reason}", detect = ["reason="])]
#[derive(TraceEvent)]
struct TraceSchedWakeup {
    #[field]
    comm: String,

    #[field]
    pid: u32,

    #[field]
    prio: i32,

    #[field(format = "{:03}")]
    target_cpu: u32,

    #[field(optional)]
    reason: Option<u32>,  // ← опциональное поле
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
