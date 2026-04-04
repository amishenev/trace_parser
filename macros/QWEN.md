# trace_parser_macros — Proc-macro для событий

## Цель

Автоматическая генерация boilerplate кода для событий трассировки. Уменьшение количества ручного кода на 80%.

---

## Синтаксис

### Базовое событие

```rust
use trace_parser_macros::TraceEvent;

#[trace_event(name = "sched_switch", aliases = ["sched_sw"])]
#[define_template("prev_comm={prev_comm} prev_pid={prev_pid} prev_prio={prev_prio} prev_state={prev_state} ==> next_comm={next_comm} next_pid={next_pid} next_prio={next_prio}")]
#[derive(TraceEvent)]
struct TraceSchedSwitch {
    #[field]
    prev_comm: String,

    #[field]
    prev_pid: u32,

    #[field]
    prev_prio: i32,

    #[field]
    prev_state: String,

    #[field]
    next_comm: String,

    #[field]
    next_pid: u32,

    #[field]
    next_prio: i32,
}
```

Тип поля автоматически выводится из Rust-типа: `String` → `string()`, `u32` → `u32()`, `bool` → `bool_int()`, `Option<T>` → optional.

### Событие с кастомным именем поля

```rust
#[trace_event(name = "cpu_frequency")]
#[define_template("state={state} cpu_id={cpu_id}")]
#[derive(TraceEvent)]
struct TraceCpuFrequency {
    #[field(name = "state")]
    current_state: u32,  // ← имя переменной отличается от имени в payload

    #[field]
    cpu_id: u32,  // ← имя совпадает
}
```

### Событие с опциональным полем

```rust
#[trace_event(name = "sched_wakeup")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu} reason={reason}")]
#[derive(TraceEvent)]
struct TraceSchedWakeup {
    #[field]
    comm: String,

    #[field]
    pid: u32,

    #[field]
    prio: i32,

    #[field]
    target_cpu: u32,

    #[field(optional)]
    reason: Option<u32>,  // ← опциональное поле
}
```

### Событие с кастомным regex

```rust
#[trace_event(name = "custom_event")]
#[define_template("code={code}")]
#[derive(TraceEvent)]
struct TraceCustomEvent {
    #[field(regex = r"[A-Z]{2}\d{3}")]
    code: String,  // ← нестандартный формат, кастомный regex
}
```

### Событие с choice (ограниченный набор значений)

```rust
#[trace_event(name = "clock_set_rate")]
#[fast_match(contains_any = ["clk=ddr_devfreq", "clk=l3c_devfreq"])]
#[define_template("clk={clk} state={state} cpu_id={cpu_id}")]
#[derive(TraceEvent)]
struct TraceDevFrequency {
    #[field(choice = ["ddr_devfreq", "l3c_devfreq"])]
    clk: String,  // ← только эти значения допустимы

    #[field]
    state: u32,

    #[field]
    cpu_id: u32,
}
```

### Enum для payload полей

```rust
use trace_parser_macros::TraceEnum;

#[derive(TraceEnum)]
pub enum PrevState {
    #[value("S")]
    Sleeping,
    #[value("R")]
    Running,
    #[value("D")]
    DiskSleep,
    #[value("X")]
    Dead,
}

// Затем в событии:
#[trace_event(name = "sched_switch")]
#[define_template("prev_state={prev_state}")]
#[derive(TraceEvent)]
struct TraceSchedSwitch {
    #[field]
    prev_state: PrevState,  // ← enum тип
}
```

### Tracing Mark событие

```rust
use trace_parser_macros::TracingMarkEvent;

#[trace_event(name = "tracing_mark_write")]
#[trace_markers("B|")]
#[define_template("B|{trace_mark_tgid}|{message}")]
#[derive(TracingMarkEvent)]
struct TraceMarkBegin {
    #[field]
    trace_mark_tgid: u32,

    #[field]
    message: String,
}

#[trace_event(name = "tracing_mark_write")]
#[trace_markers("B|", "ReceiveVsync")]
#[define_template("{?ignore:extra_info}ReceiveVsync {frame_number}")]
#[derive(TracingMarkEvent)]
struct TraceReceiveVsync {
    #[field]
    frame_number: u32,
}
```

---

## Атрибуты

### `#[trace_event(name, aliases, skip_registration)]`

| Параметр | Обязательный | Описание |
|----------|--------------|----------|
| `name` | ✅ | Имя события (event_name) |
| `aliases` | ❌ | Алиасы для event_name |
| `skip_registration` | ❌ | Пропустить регистрацию — для событий, обрабатываемых явно (TraceMarkBegin/End) |

### `#[trace_markers("...", "...")]`

Маркеры для быстрой проверки payload через SIMD.

### `#[fast_match(contains_any = ["...", ...])]`

Опционально: `FastMatch::payload_quick_check` через `contains_any(line, ...)`. Если атрибута нет — только `PAYLOAD_MARKERS` / дефолтная проверка.

### `#[define_template("...")]`

Шаблон payload. Можно указать несколько для разных форматов.

#### Явное указание id (опционально)

```rust
// Явный id=0
#[define_template("comm={comm} pid={pid}", id = 0)]
// id=1 по умолчанию (второй темплейт)
#[define_template("comm={comm} pid={pid} reason={reason}")]
```

**Правила auto-assign:**
- Первый темплейт без `id` → `id = 0`
- Второй темплейт без `id` → `id = 1`
- Явный `id = N` → использует N
- Auto-assign продолжает после максимального явного id

### `#[field(...)]`

| Атрибут | Тип | Обязательный | Описание |
|---------|-----|--------------|----------|
| `name` | string | ❌ | Имя в payload (по умолчанию = имя поля) |
| `regex` | string | ❌ | Кастомный regex для парсинга |
| `choice` | array | ❌ | Ограниченный набор значений |
| `format` | string | ❌ | Формат-строка для рендера (напр. `"{:03}"`) |
| `optional` | flag | ❌ | Поле опционально (Option<T>) |
| `readonly` | flag | ❌ | Только чтение в Python |
| `private` | flag | ❌ | Не экспортировать в Python |

### Автоматический вывод типов

| Rust тип | FieldSpec | Описание |
|----------|-----------|----------|
| `String` | `string()` | Строка |
| `u8/u16/u32` | `u32()` | Беззнаковое |
| `u64` | `custom(r"\d+")` | Большое беззнаковое |
| `i8/i16/i32` | `i32()` | Знаковое |
| `i64` | `custom(r"-?\d+")` | Большое знаковое |
| `f32/f64` | `f64()` | Float |
| `bool` | `bool_int()` | Булево как 0/1 |
| `Option<T>` | auto | Опциональное поле |

---

## Что генерируется

### Для `#[derive(TraceEvent)]`

```rust
// 1. impl EventType
impl ::trace_parser::common::EventType for TraceSchedSwitch {
    const EVENT_NAME: &'static str = "sched_switch";
    const EVENT_ALIASES: &'static [&'static str] = &["sched_sw"];
}

// 2. impl FastMatch
impl ::trace_parser::common::FastMatch for TraceSchedSwitch {
    const PAYLOAD_MARKERS: &'static [&'static [u8]] = &[];
}

// 3. impl TemplateEvent
impl ::trace_parser::common::TemplateEvent for TraceSchedSwitch {
    fn formats() -> &'static FormatRegistry { ... }
    fn detect_format(_payload: &str) -> u8 { 0 }
    fn parse_payload(...) -> Option<Self> { ... }
    fn render_payload(&self) -> PyResult<String> { ... }
}

// 4. #[pymethods]
#[pyo3::pymethods]
impl TraceSchedSwitch {
    #[new]
    fn new(thread_name: String, thread_tid: u32, ..., prev_comm: String, ...) -> PyResult<Self> { ... }

    #[staticmethod]
    fn can_be_parsed(line: &str) -> bool { ... }

    #[staticmethod]
    fn parse(line: &str) -> Option<Self> { ... }

    fn to_string(&self) -> PyResult<String> { ... }

    #[getter]
    fn payload(&self) -> PyResult<String> { ... }

    #[getter]
    fn template(&self) -> &'static str { ... }

    // + геттеры/сеттеры для полей
}

// 5. Регистрация
::trace_parser::register_parser!("sched_switch", TraceSchedSwitch);
```

### Для `#[derive(TracingMarkEvent)]`

Всё то же самое + регистрация через `register_tracing_mark_parser!`.

### Для `#[derive(TraceEnum)]`

```rust
impl ::std::fmt::Display for PrevState {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match self {
            Self::Sleeping => f.write_str("S"),
            Self::Running => f.write_str("R"),
            // ...
        }
    }
}

impl ::std::str::FromStr for PrevState {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "S" => Ok(Self::Sleeping),
            "R" => Ok(Self::Running),
            // ...
            _ => Err(format!("invalid PrevState: {}", s)),
        }
    }
}

impl ::trace_parser::payload_template::TraceEnum for PrevState {
    fn values() -> &'static [&'static str] {
        &["S", "R", "D", "X"]
    }
}
```

---

## Ограничения

1. **Только именованные поля** — `struct NamedFields { field: Type }`
2. **Базовые поля** — `thread_name`, `thread_tid`, etc. должны быть объявлены вручную
3. **Порядок полей** — не важен, макрос обработает в любом порядке
4. **Несколько темплейтов** — детекция формата реализована через `detect_format()`
5. **`#[pyclass]`** — должен быть указан вручную для PyO3 совместимости

```rust
#[derive(TraceEvent)]
#[pyclass]  // ← требуется для Python API
pub struct TraceMyEvent {
    // ...
}
```
