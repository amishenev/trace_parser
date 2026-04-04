# Пример: Ограниченный набор значений через `#[field(choice = [...])]`

Использование `#[field(choice = [...])]` для полей с фиксированным набором значений.

```rust
use pyo3::prelude::*;
use trace_parser_macros::TraceEvent;

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
#[derive(TraceEvent)]
#[trace_event(name = "clock_set_rate")]
#[define_template("clk={clk} state={state}")]
struct TraceClockRate {
    #[field]
    format_id: u8,
    #[pyo3(get, set)]
    #[field]
    pub thread_name: String,
    #[pyo3(get, set)]
    #[field]
    pub thread_tid: u32,
    #[pyo3(get, set)]
    #[field]
    pub thread_tgid: u32,
    #[pyo3(get, set)]
    #[field]
    pub cpu: u32,
    #[pyo3(get, set)]
    #[field]
    pub flags: String,
    #[pyo3(get, set)]
    #[field]
    pub timestamp: f64,
    #[pyo3(get)]
    #[field]
    pub event_name: String,
    // Принимает только указанные значения
    #[field(choice = ["ddr_devfreq", "l3c_devfreq", "gpu"])]
    pub clk: String,

    #[field]
    pub state: u32,
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
