# Пример: Быстрая проверка через `#[fast_match]`

Использование `#[fast_match(contains_any = [...])]` для SIMD-проверки payload через `memchr::memmem::find`.

```rust
use trace_parser_macros::TraceEvent;

#[trace_event(name = "clock_set_rate")]
#[fast_match(contains_any = ["clk=ddr_devfreq", "clk=l3c_devfreq"])]
#[define_template("clk={clk} state={state} cpu_id={cpu_id}")]
#[derive(TraceEvent)]
struct TraceDevFrequency {
    #[field(choice = ["ddr_devfreq", "l3c_devfreq"])]
    clk: String,

    #[field]
    state: u32,

    #[field]
    cpu_id: u32,
}
```

## Что генерируется

```rust
impl FastMatch for TraceDevFrequency {
    const PAYLOAD_MARKERS: &'static [&'static [u8]] = &[];

    fn payload_quick_check(line: &str) -> bool {
        ::trace_parser::common::contains_any(line, &["clk=ddr_devfreq", "clk=l3c_devfreq"])
    }
}
```

## Зачем

`payload_quick_check` вызывается **до** regex-парсинга. Если строка не содержит ни одного из маркеров — парсинг не запускается. Это ускоряет обработку больших логов.

## Использование

```rust
// Содержит "clk=ddr_devfreq" → пройдёт быструю проверку
let line = "task-1 (1) [0] .... 1.0: clock_set_rate: clk=ddr_devfreq state=100000 cpu_id=0";
assert!(TraceDevFrequency::can_be_parsed(line));

// Не содержит маркеров → не пройдёт
let line2 = "task-1 (1) [0] .... 1.0: clock_set_rate: clk=gpu state=100 cpu_id=0";
assert!(!TraceDevFrequency::can_be_parsed(line2));
```
