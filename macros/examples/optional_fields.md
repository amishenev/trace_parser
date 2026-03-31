# Пример: Опциональные поля

Использование `#[field(optional)]` для полей, которые могут отсутствовать.

```rust
use trace_parser_macros::TraceEvent;

#[trace_event(name = "sched_wakeup")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu} reason={reason}")]
#[derive(TraceEvent)]
struct TraceSchedWakeup {
    #[field(ty = "string")]
    comm: String,

    #[field(ty = "u32")]
    pid: u32,

    #[field(ty = "i32")]
    prio: i32,

    #[field(ty = "u32")]
    target_cpu: u32,

    #[field(ty = "u32", optional)]
    reason: Option<u32>,  // ← опциональное поле
}

// Кастомная проверка для детекции формата с reason
impl ::trace_parser::common::FastMatch for TraceSchedWakeup {
    fn payload_quick_check(line: &str) -> bool {
        line.contains("reason=")
    }
}
```

## Что генерируется

- `reason` имеет тип `Option<u32>` в Rust
- В Python поле может быть `None`
- Конструктор принимает `Option<u32>` для `reason`

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
