# Пример: Несколько темплейтов с SIMD-детекцией

Использование нескольких `#[define_template]` с автоматической детекцией формата через `detect = [...]`.

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

    #[field]
    reason: Option<u32>,
}
```

## Как работает

1. Макрос генерирует `FormatRegistry` с двумя темплейтами:
   - `kind = 0` → `"comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}"`
   - `kind = 1` → `"comm={comm} pid={pid} prio={prio} target_cpu={target_cpu} reason={reason}"`

2. При парсинге вызывается `detect_format(payload)` с SIMD-проверкой:
   ```rust
   fn detect_format(payload: &str) -> u8 {
       const MARKERS: &'static [(&[u8], u8)] = &[(b"reason=", 1)];
       for (marker, id) in MARKERS {
           if memchr::memmem::find(payload.as_bytes(), *marker).is_some() {
               return *id;
           }
       }
       0
   }
   ```

3. `parse_payload()` использует темплейт с нужным `format_id`

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
