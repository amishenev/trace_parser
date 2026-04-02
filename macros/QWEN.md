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

### Событие с кастомным именем поля

```rust
#[trace_event(name = "cpu_frequency")]
#[define_template("state={state} cpu_id={cpu_id}")]
#[derive(TraceEvent)]
struct TraceCpuFrequency {
    #[field(ty = "u32", name = "state")]
    current_state: u32,  // ← имя переменной отличается от имени в payload
    
    #[field(ty = "u32")]
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
    #[field(ty = "string")]
    comm: String,
    
    #[field(ty = "u32")]
    pid: u32,
    
    #[field(ty = "u32")]
    prio: i32,
    
    #[field(ty = "u32")]
    target_cpu: u32,
    
    #[field(ty = "u32", optional)]
    reason: Option<u32>,  // ← опциональное поле
}

// Кастомная проверка payload (если нужно)
impl FastMatch for TraceSchedWakeup {
    fn payload_quick_check(line: &str) -> bool {
        line.contains("reason=")
    }
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
    #[field(ty = "u32")]
    trace_mark_tgid: u32,
    
    #[field(ty = "string")]
    message: String,
}

#[trace_event(name = "tracing_mark_write")]
#[trace_markers("B|", "ReceiveVsync")]
#[define_template("{?ignore:extra_info}ReceiveVsync {frame_number}")]
#[derive(TracingMarkEvent)]
struct TraceReceiveVsync {
    #[field(ty = "u32")]
    frame_number: u32,
}
```

---

## Атрибуты

### `#[trace_event(name, aliases)]`

| Параметр | Обязательный | Описание |
|----------|--------------|----------|
| `name` | ✅ | Имя события (event_name) |
| `aliases` | ❌ | Алиасы для event_name |

### `#[trace_markers("...", "...")]`

Маркеры для быстрой проверки payload через SIMD.

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

**Примеры:**
```rust
// [None, None] → [0, 1]
#[define_template("a={a}")]
#[define_template("a={a} b={b}")]

// [Some(0), None] → [0, 1]
#[define_template("a={a}", id = 0)]
#[define_template("a={a} b={b}")]

// [Some(1), Some(2), None] → [1, 2, 3]
#[define_template("a={a}", id = 1)]
#[define_template("a={a} b={b}", id = 2)]
#[define_template("a={a} b={b} c={c}")]

// [Some(0), None, Some(5), None] → [0, 1, 5, 6]
#[define_template("a={a}", id = 0)]
#[define_template("a={a} b={b}")]
#[define_template("a={a} b={b} c={c}", id = 5)]
#[define_template("a={a} b={b} c={c} d={d}")]
```

### `#[field(...)]`

| Атрибут | Тип | Обязательный | Описание |
|---------|-----|--------------|----------|
| `ty` | string | ✅ | Тип парсинга |
| `name` | string | ❌ | Имя в payload (по умолчанию = имя поля) |
| `optional` | flag | ❌ | Поле опционально (Option<T>) |
| `readonly` | flag | ❌ | Только чтение в Python |
| `private` | flag | ❌ | Не экспортировать в Python |

### Поддерживаемые типы (`ty`)

| `ty` | Rust тип | Описание |
|------|----------|----------|
| `"string"` | `String` | Строка |
| `"u32"` | `u32` | Беззнаковое 32-bit |
| `"i32"` | `i32` | Знаковое 32-bit |
| `"f64"` | `f64` | Float 64-bit |
| `"bool_int"` | `bool` | Булево как 0/1 |

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
