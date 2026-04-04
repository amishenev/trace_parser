# Пример: Кастомное имя поля

Использование `#[field(name = "...")]` для маппинга имён.

```rust
use trace_parser_macros::TraceEvent;

#[trace_event(name = "cpu_frequency")]
#[define_template("state={state} cpu_id={cpu_id}")]
#[derive(TraceEvent)]
struct TraceCpuFrequency {
    #[field(name = "state")]
    current_state: u32,  // ← имя переменной != имя в payload

    #[field]
    cpu_id: u32,  // ← имя совпадает
}
```

## Что генерируется

- `impl TemplateEvent` с маппингом `current_state` → `state`
- В `parse_payload()` поле `state` из шаблона маппится на `current_state`
- В `render_payload()` поле `current_state` рендерится как `state`

## Использование

```rust
let event = TraceCpuFrequency::new(
    "swapper".to_string(), 0, 0, 0, "....".to_string(), 12345.678900,
    "cpu_frequency".to_string(),
    933000000, 0,
).unwrap();

// payload будет "state=933000000 cpu_id=0"
println!("{}", event.payload().unwrap());
```
