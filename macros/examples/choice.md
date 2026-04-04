# Пример: Ограниченный набор значений через `#[field(choice = [...])]`

Использование `#[field(choice = [...])]` для полей с фиксированным набором значений.

```rust
use trace_parser_macros::TraceEvent;

#[trace_event(name = "clock_set_rate")]
#[define_template("clk={clk} state={state}")]
#[derive(TraceEvent)]
struct TraceClockRate {
    // Принимает только указанные значения
    #[field(choice = ["ddr_devfreq", "l3c_devfreq", "gpu"])]
    clk: String,

    #[field]
    state: u32,
}
```

## Что генерируется

```rust
// В FieldSpec:
FieldSpec::choice(&["ddr_devfreq", "l3c_devfreq", "gpu"])
```

В regex шаблона это превращается в `(?:ddr_devfreq|l3c_devfreq|gpu)`.

## Зачем

1. **Валидация при парсинге** — regex принимает только указанные значения
2. **Документация** — из кода видно допустимые значения
3. **Генерация шаблона** — `FieldSpec::choice` используется для построения regex

## Использование

```rust
// Валидное значение
let line = "task-1 (1) [0] .... 1.0: clock_set_rate: clk=ddr_devfreq state=100000";
let event = TraceClockRate::parse(line).unwrap();
assert_eq!(event.clk, "ddr_devfreq");

// Невалидное значение → парсинг вернёт None
let line2 = "task-1 (1) [0] .... 1.0: clock_set_rate: clk=unknown_clk state=100";
assert!(TraceClockRate::parse(line2).is_none());
```
