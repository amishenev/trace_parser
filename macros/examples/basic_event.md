# Пример: Базовое событие

Использование `#[derive(TraceEvent)]` для простого события без опциональных полей.

```rust
use trace_parser_macros::TraceEvent;

#[trace_event(name = "sched_switch")]
#[define_template(
    "prev_comm={prev_comm} prev_pid={prev_pid} prev_prio={prev_prio} prev_state={prev_state} ==> next_comm={next_comm} next_pid={next_pid} next_prio={next_prio}"
)]
#[derive(TraceEvent)]
struct TraceSchedSwitch {
    #[field(ty = "string")]
    prev_comm: String,

    #[field(ty = "u32")]
    prev_pid: u32,

    #[field(ty = "i32")]
    prev_prio: i32,

    #[field(ty = "string")]
    prev_state: String,

    #[field(ty = "string")]
    next_comm: String,

    #[field(ty = "u32")]
    next_pid: u32,

    #[field(ty = "i32")]
    next_prio: i32,
}
```

## Что генерируется

- `impl EventType` с `EVENT_NAME = "sched_switch"`
- `impl FastMatch` с пустыми маркерами
- `impl TemplateEvent` с одним форматом
- `#[pymethods]` с конструктором, `parse()`, `to_string()`, геттерами
- Регистрация через `register_parser!("sched_switch", TraceSchedSwitch)`

## Использование

```rust
let event = TraceSchedSwitch::new(
    "bash".to_string(), 1234, 1234, 0, "....".to_string(), 12345.678901,
    "sched_switch".to_string(),
    "bash".to_string(), 1977, 120, "S".to_string(),
    "worker".to_string(), 123, 120,
).unwrap();

println!("{}", event.to_string().unwrap());
// bash-1234 (1234) [000] .... 12345.678901: sched_switch: prev_comm=bash ...
```
