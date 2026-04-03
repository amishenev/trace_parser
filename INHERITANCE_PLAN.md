# План: честное наследование через PyO3 `extends`

## Цель

Избавиться от дублирования базовых полей во всех событиях и построить честную иерархию наследования.

## Текущая проблема

Каждый типизированный класс дублирует 8 базовых полей:
- `thread_name`, `thread_tid`, `thread_tgid`, `cpu`, `flags`, `timestamp`, `event_name`, `format_id`

Это boilerplate, источник ошибок, и архитектурно некорректно.

## Целевая иерархия

```
Trace (базовый, fallback, payload_raw)
├── TraceSchedSwitch
├── TraceSchedWakeup
├── TraceSchedWakeupNew
├── TraceSchedProcessExit
├── TraceCpuFrequency
├── TraceDevFrequency
├── TraceExit
└── TracingMark (тоже наследуется от Trace, payload_raw без темплейта)
    ├── TraceMarkBegin
    │   └── TraceReceiveVsync
    └── TraceMarkEnd
```

## PyO3 наследование

PyO3 поддерживает `#[pyclass(extends = Base)]` — настоящее наследование на уровне Python:

```python
issubclass(TraceSchedSwitch, Trace)  # True
event.prev_comm         # поле подкласса
event.thread_name       # унаследованное поле
```

### Как это работает

```rust
// Trace — базовый класс, без extends
#[pyclass]
pub struct Trace {
    pub thread_name: String,
    pub thread_tid: u32,
    // ... все 8 базовых полей + payload_raw
}

// TraceSchedSwitch — наследуется от Trace
#[pyclass(extends = Trace)]
pub struct TraceSchedSwitch {
    // Только payload-поля, базовые — из Trace
    pub prev_comm: String,
    pub prev_pid: u32,
    // ...
}
```

### Доступ к полям в Rust

```rust
let sched_switch: PyRef<TraceSchedSwitch> = ...;
let base: PyRef<Trace> = sched_switch.as_ref();  // базовые поля
let prev_comm = &sched_switch.prev_comm;          // свои поля
```

### Доступ в Python

Прозрачный — Python видит плоский объект:
```python
event.thread_name  # работает напрямую
event.prev_comm    # работает напрямую
```

---

## Синтаксис макроса

### Базовый класс (fallback)

```rust
#[trace_event(name = "unknown", fallback)]
#[derive(TraceEvent)]
pub struct Trace {
    #[pyo3(get, set)]
    #[field(ty = "string")]
    pub thread_name: String,
    // ... все базовые поля + payload_raw
}
```

`fallback` → генерирует только `EventType` + `FastMatch` + `pymethods`, **без** `TemplateEvent`.

### Наследник

```rust
#[trace_event(name = "sched_switch", extends = "Trace")]
#[define_template("prev_comm={prev_comm} prev_pid={prev_pid} ...")]
#[derive(TraceEvent)]
pub struct TraceSchedSwitch {
    // Только payload-поля
    #[pyo3(get, set)]
    #[field(ty = "string")]
    pub prev_comm: String,
    // ...
}
```

`extends = "Trace"` → генерирует `#[pyclass(extends = Trace)]`.

### Цепочка наследования

```rust
// TracingMark наследуется от Trace
#[trace_event(name = "tracing_mark_write", extends = "Trace")]
pub struct TracingMark { ... }

// TraceMarkBegin наследуется от TracingMark
#[trace_event(name = "tracing_mark_write", extends = "TracingMark")]
#[trace_markers("B|")]
pub struct TraceMarkBegin { ... }
```

---

## Что генерирует макрос

### 1. `#[pyclass(extends = Base)]`

```rust
// Было:
#[pyclass]
pub struct TraceSchedSwitch { ... }

// Стало:
#[pyclass(extends = Trace)]
pub struct TraceSchedSwitch {
    // Только payload-поля
}
```

### 2. Конструктор — `PyClassInitializer`

```rust
// Было:
fn new(...) -> PyResult<Self> { ... }

// Стало:
fn new(...) -> PyResult<PyClassInitializer<Self>> {
    let base = Trace::new(thread_name, thread_tid, ...)?;
    Ok(PyClassInitializer::from(base).add_subclass(TraceSchedSwitch {
        prev_comm, prev_pid, ...
    }))
}
```

### 3. `parse_payload` — только payload-поля

```rust
fn parse_payload(parts: BaseTraceParts, captures: &Captures, format_id: u8) -> Option<Self> {
    let base = Trace::from_parts(parts);
    let subclass = Self {
        prev_comm: cap_str(captures, "prev_comm")?,
        prev_pid: cap_parse(captures, "prev_pid")?,
        // ...
    };
    Some(PyClassInitializer::from(base).add_subclass(subclass))
}
```

### 4. `to_string()` — через `as_ref()` к базовым полям

```rust
fn to_string(&self) -> PyResult<String> {
    let base: &Trace = self.as_ref();
    let payload = self.render_payload()?;
    Ok(format_trace_header(
        &base.thread_name, base.thread_tid, ..., &payload
    ))
}
```

---

## Этапы реализации

1. **macros/src/attrs.rs** — парсинг `extends = "TypeName"`, `fallback`
2. **macros/src/generator.rs** — генерация с `extends`:
   - `#[pyclass(extends = ...)]`
   - конструкторы с `PyClassInitializer`
   - `parse_payload` с инициализацией base + subclass
   - `to_string()` через `as_ref()` к базовым полям
   - fallback режим: без `TemplateEvent`
3. **macros/src/lib.rs** — передача `extends` в генераторы
4. **src/trace.rs** — базовый класс с `fallback`, без `extends`
5. **src/sched_switch.rs** — `extends = "Trace"`, убрать базовые поля
6. **Остальные события** — `extends = "Trace"`, убрать базовые поля
7. **TracingMark и подтипы** — цепочка наследования
8. **Тесты** — убедиться что всё работает
9. **Бенчмарки** — проверить регрессию

---

## Перформанс

| Операция | Было | Стало | Разница |
|----------|------|-------|---------|
| Создание Py-объекта | 1 инициализация | 2 инициализации в 1 PyCell | ~5-10% |
| Доступ к полям из Python | Прямой | Прямой (Python descriptor) | 0% |
| Парсинг (regex) | regex → struct | regex → struct | 0% |
| `to_string()` | Прямой доступ | `as_ref()` к base | ~1% |

**Вывод:** Накладные расходы минимальны, в пределах погрешности бенчмарков.

---

## Риски

1. **Сложнее конструкторы** — `PyClassInitializer::new((child,), (base,), py)`
2. **Доступ в Rust менее удобен** — `as_ref()` вместо прямого
3. **Много файлов затронуть** — все типизированные события
4. **PyO3 `extends`** — требует аккуратной работы с `PyRef` / `PyRefMut`

---

## Статус

⏸️ Отложено. Макрос уже работает для плоской структуры, наследование — следующий крупный шаг.
