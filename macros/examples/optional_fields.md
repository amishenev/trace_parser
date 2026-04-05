# Пример: Option-поля и multi-template

Опциональность теперь выводится из типа поля `Option<T>`. `#[field(optional)]` больше не нужен.

```rust
use trace_parser_macros::trace_event_class;

#[trace_event_class]
#[trace_event(name = "sched_wakeup")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu} reason={reason}", detect = ["reason="])]
struct TraceSchedWakeup {
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
    pub comm: String,
    #[field]
    pub pid: u32,
    #[field]
    pub prio: i32,
    #[field(format = "{:03}")]
    pub target_cpu: u32,

    #[field]
    pub reason: Option<u32>,
}
```

## Что важно

- `reason: Option<u32>` автоматически парсится/рендерится как optional
- в Python это поле принимает `int | None`
- `detect = ["reason="]` используется для быстрого выбора формата
