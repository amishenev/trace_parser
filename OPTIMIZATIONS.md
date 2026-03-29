# trace_parser — План оптимизаций

## Приоритет 1: Производительность (критично для больших файлов)

### 1.1 Dispatch таблица для `parse_trace()` — O(1) вместо O(n) ✅ ВЫПОЛНЕНО

**Проблема:** 10+ последовательных проверок `can_be_parsed()` для каждой строки.

**Решение:** Match по `event_name`:

```rust
// src/dispatch.rs
fn extract_event_name(line: &str) -> Option<&str> {
    let colon_pos = line.find(": ")? + 2;
    let rest = &line[colon_pos..];
    let end_pos = rest.find(": ")?;
    Some(rest[..end_pos].trim())
}

fn parse_and_wrap<T: IntoPy<PyObject>>(py: Python<'_>, line: &str, parser: fn(&str) -> Option<T>) -> Option<PyObject> {
    parser(line).map(|e| e.into_py(py))
}

pub fn dispatch_parse(py: Python<'_>, line: &str) -> PyResult<Option<PyObject>> {
    let Some(event_name) = extract_event_name(line) else { return Ok(None); };

    let result = match event_name {
        "tracing_mark_write" => parse_tracing_mark(py, line),
        "clock_set_rate" => parse_and_wrap(py, line, TraceDevFrequency::parse),
        "cpu_frequency" => parse_and_wrap(py, line, TraceCpuFrequency::parse),
        "sched_switch" => parse_and_wrap(py, line, TraceSchedSwitch::parse),
        "sched_wakeup" => parse_and_wrap(py, line, TraceSchedWakeup::parse),
        "sched_wakeup_new" => parse_and_wrap(py, line, TraceSchedWakeupNew::parse),
        "sched_process_exit" => parse_and_wrap(py, line, TraceSchedProcessExit::parse),
        _ => parse_and_wrap(py, line, Trace::parse),
    };

    Ok(result)
}
```

**Выгода:** O(1) dispatch вместо O(n) проверок

---

### 1.2 Оптимизация `render_payload()` — убрать HashMap аллокации

**Проблема:** `HashMap::from([...])` создаёт аллокацию на каждый `to_string()`.

**Решение:** Использовать `smallvec` или массив для маленьких шаблонов:

```rust
// Вариант A: smallvec
use smallvec::{smallvec, SmallVec};

let values: SmallVec<[(&str, TemplateValue); 8]> = smallvec![
    ("prev_comm", TemplateValue::Str(&self.prev_comm)),
    ("prev_pid", TemplateValue::U32(self.prev_pid)),
    // ...
];

// Вариант B: array + итерация
let values = [
    ("prev_comm", TemplateValue::Str(&self.prev_comm)),
    // ...
];
template.format_iter(values.iter())
```

**Выгода:** 2-3x быстрее `to_string()`, меньше аллокаций

---

### 1.3 Streaming API для больших файлов

**Проблема:** Нет возможности парсить файлы >1GB без загрузки в память.

**Решение:** Python уже умеет читать по строке — не нужно делать Rust итератор.

**Вместо этого:**
```python
# Python уже streaming
with open("trace.txt") as f:
    for line in f:
        event = parse_trace(line)
```

**Что стоит сделать в Rust:**
- `parse_trace_file()` функция для массового парсинга с фильтрацией
- Меньше FFI вызовов (1 вместо миллионов)

---

### 1.4 SIMD оптимизации

**memchr** — SIMD поиск подстроки (вместо `line.find(": ")`)
**lexical-core** — SIMD парсинг чисел (вместо `str.parse()`)

**Пример использования:**
```rust
use memchr::memmem;
use lexical_core::parse;

fn extract_event_name(line: &str) -> Option<&str> {
    let pos = memmem::find(line.as_bytes(), b": ")? + 2;
    // ...
}

// В парсинге чисел:
tid: parse(captures.name("tid")?.as_str().as_bytes()).ok()?,
```

**Ожидаемая выгода:** ~30-50% быстрее парсинг каждой строки

**TODO: Обновить README.md** — добавить секцию "Performance" с:
- Описанием SIMD оптимизаций (memchr, lexical-core)
- Примером `parse_trace_file()` для массового парсинга
- Бенчмарками до/после

---

## Приоритет 2: Расширяемость (удобство добавления событий)

### 2.1 Proc-macro для событий

**Проблема:** Boilerplate — каждое событие дублирует ~100 строк кода.

**Решение:** Макрос `#[trace_event]`:

```rust
#[trace_event(
    name = "sched_switch",
    template = "prev_comm={prev_comm} prev_pid={prev_pid} ..."
)]
struct TraceSchedSwitch {
    base: Trace,
    prev_comm: String,
    prev_pid: u32,
    prev_prio: i32,
    prev_state: String,
    next_comm: String,
    next_pid: u32,
    next_prio: i32,
}
```

**Генерирует автоматически:**
- `impl EventType` с `EVENT_NAME`
- `impl FastMatch` с `quick_check()`
- `impl TemplateEvent` с `parse_payload()`, `render_payload()`
- `#[pymethods]` с `can_be_parsed()`, `parse()`, `to_string()`

**Выгода:** 80% меньше кода на событие

---

### 2.2 Автоматическая регистрация в dispatch таблице

**Проблема:** Нужно вручную добавлять каждое событие в `dispatch_parse()`.

**Решение:** Использовать `inventory` crate для авто-регистрации:

```rust
inventory::submit! {
    RegisteredParser {
        event_name: "sched_switch",
        parser: ParseFn::new::<TraceSchedSwitch>(),
    }
}
```

**Или через макрос:**
```rust
#[trace_event(name = "sched_switch", register = true)]
```

---

### 2.3 Flattened access для Python

**Проблема:** `vsync.begin.mark.base.timestamp` вместо `vsync.timestamp`.

**Решение A:** `__getattr__` proxy:

```rust
#[pymethods]
impl TraceReceiveVsync {
    fn __getattr__(&self, name: &str) -> PyResult<PyObject> {
        // Proxy для base.* полей
        match name {
            "timestamp" => Ok(self.begin.mark.base.timestamp.into_py(py)),
            "thread_name" => Ok(self.begin.mark.base.thread_name.clone().into_py(py)),
            // ...
            _ => Err(PyAttributeError::new_err(...)),
        }
    }
}
```

**Решение B:** Макрос для генерации proxy:

```rust
#[pyproxy(fields(timestamp, thread_name, tid, cpu))]
pub struct TraceReceiveVsync {
    begin: TraceMarkBegin,
    frame_number: u32,
}
```

---

## Приоритет 3: Современные паттерны

### 3.1 Обновление до Rust 2024 + LazyLock

**Текущее:** Rust 2021 + `once_cell::sync::Lazy` + `Box::leak`

**Целевое:** Rust 2024 + `std::sync::LazyLock` (стабилизировано в 1.80)

```rust
use std::sync::LazyLock;

static FORMATS: LazyLock<FormatRegistry> = LazyLock::new(|| {
    FormatRegistry::new(vec![...])
});
```

**Выгода:** Нет `Box::leak`, нет утечек памяти

---

### 3.2 Кэширование в CI

**Добавить в `.github/workflows/ci.yml`:**

```yaml
- name: Cache cargo
  uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/registry
      ~/.cargo/git
      target
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

**Выгода:** 2-3x быстрее CI

---

### 3.3 Интеграционные тесты

**Добавить:**
- Тесты на реальных ftrace логах (10MB+)
- Тесты производительности в CI (`cargo bench` с порогом)
- Snapshot тесты для `to_string()` round-trip

---

## Приоритет 4: Дополнительные возможности

### 4.1 Сериализация в форматы аналитики

**Добавить:**
- `to_json()` / `from_json()` — для веб-интерфейсов
- `to_arrow()` — для pandas/polars интеграции
- `to_parquet()` — для хранения больших трейсов

---

### 4.2 Фильтрация и агрегация

**Добавить Python API:**

```python
from trace_parser import TraceFilter, TraceAggregator

# Фильтрация
events = TraceFilter()
    .from_file("trace.txt")
    .where_event("sched_switch")
    .where_cpu(0, 1, 2)
    .where_timestamp_range(100.0, 200.0)
    .collect()

# Агрегация
agg = TraceAggregator()
    .group_by("event_name")
    .count()
    .run(events)
```

---

## Roadmap

### Краткосрочная (1-2 недели)
- [x] Dispatch таблица для `parse_trace()` (#1.1)
- [x] HashMap → Array в `render_payload()` (#1.2)
- [x] Кэширование в CI (#3.2)
- [x] `parse_trace_file()` для массового парсинга (#1.3)
- [ ] SIMD оптимизации (#1.4) — memchr + lexical-core

### Среднесрочная (1-2 месяца)
- [ ] Proc-macro для событий (#2.1)
- [ ] Flattened access (#2.3)

### Долгосрочная (3-6 месяцев)
- [ ] Сериализация в Arrow/Parquet (#4.1)
- [ ] Фильтрация и агрегация (#4.2)
- [ ] Обновление до Rust 2024 (#3.1)

---

## Бенчмарки для валидации

### Текущие референсы (sched_switch positive)

| Метод | Время |
|-------|-------|
| `Trace::can_be_parsed` | ~319 ns/op |
| `TraceSchedSwitch::can_be_parsed` | ~116 ns/op |
| `TraceSchedSwitch::parse()` | ~9.7 μs/op |

### Целевые показатели после оптимизаций

| Метод | Цель |
|-------|------|
| `parse_trace()` dispatch | <50 ns/op |
| `to_string()` | <1 μs/op |
| Streaming throughput | >100 MB/s |
