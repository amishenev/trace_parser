# Пример: Несколько темплейтов с `detect_format`

Использование нескольких `#[define_template]` с кастомной логикой выбора формата.

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
    reason: Option<u32>,
}

// Кастомная логика выбора формата
impl ::trace_parser::common::TemplateEvent for TraceSchedWakeup {
    fn detect_format(payload: &str) -> u8 {
        if payload.contains("reason=") {
            1  // Второй темплейт (с reason)
        } else {
            0  // Первый темплейт (без reason)
        }
    }
}
```

## Как работает

1. Макрос генерирует `FormatRegistry` с двумя темплейтами:
   - `kind = 0` → `"comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}"`
   - `kind = 1` → `"comm={comm} pid={pid} prio={prio} target_cpu={target_cpu} reason={reason}"`

2. При парсинге вызывается `detect_format(payload)`:
   - Если payload содержит `"reason="` → возвращается `1`
   - Иначе → возвращается `0`

3. `parse_payload()` использует темплейт с нужным `format_id`

## Проблема текущего дизайна

**Сейчас:** `detect_format()` по умолчанию возвращает `0`. Нужно переопределять вручную.

**План:** Автоматическая детекция по наличию полей в payload (TODO).

```rust
// Будущая автоматическая детекция
fn detect_format(payload: &str) -> u8 {
    // Если есть "reason=" → формат 1, иначе → формат 0
    if payload.contains("reason=") { 1 } else { 0 }
}
```

## Использование

```rust
// Формат 0 (без reason)
let line1 = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000";
let event1 = TraceSchedWakeup::parse(line1).unwrap();
assert_eq!(event1.reason, None);

// Формат 1 (с reason)
let line2 = "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000 reason=3";
let event2 = TraceSchedWakeup::parse(line2).unwrap();
assert_eq!(event2.reason, Some(3));
```
