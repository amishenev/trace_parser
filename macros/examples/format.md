# Пример: Кастомный формат через `#[field(format = "...")]`

Использование `#[field(format = "...")]` для кастомного рендера payload.

```rust
use trace_parser_macros::TraceEvent;

#[trace_event(name = "sched_wakeup")]
#[define_template("comm={comm} pid={pid} target_cpu={target_cpu}")]
#[derive(TraceEvent)]
struct TraceSchedWakeup {
    #[field]
    comm: String,

    #[field]
    pid: u32,

    // Рендерится как "000", "001", "012" и т.д.
    #[field(format = "{:03}")]
    target_cpu: u32,
}
```

## Что генерируется

```rust
fn render_payload(&self) -> PyResult<String> {
    let template = Self::formats().template(self.format_id).unwrap();
    let values = &[
        ("comm", Some(TemplateValue::Str(&self.comm))),
        ("pid", Some(TemplateValue::U32(self.pid))),
        ("target_cpu", Some(TemplateValue::Str(&format!("{:03}", self.target_cpu)))),
    ];
    template.format(values)
}
```

## Зачем

Без `format` поле `target_cpu: u32` рендерилось бы как `0`, `1`, `12`. С `format = "{:03}"` — как `000`, `001`, `012`, что соответствует формату трассировки.

## Поддерживаемые форматы

Любая строка-формат Rust:

| Атрибут | Результат для `42` |
|---------|-------------------|
| `format = "{:03}"` | `042` |
| `format = "{:06}"` | `000042` |
| `format = "{:#x}"` | `0x2a` |
| `format = "{:.2}"` | `42.00` (для float) |

## Использование

```rust
let event = TraceSchedWakeup::new(
    "kworker".to_string(), 123, 123, 0, "....".to_string(), 12345.679001,
    "sched_wakeup".to_string(),
    "bash".to_string(), 1977, 7,
).unwrap();

assert_eq!(event.payload().unwrap(), "comm=bash pid=1977 target_cpu=007");
```
