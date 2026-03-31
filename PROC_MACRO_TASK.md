# Proc-macro для событий — План реализации

## Цель
Уменьшить boilerplate код для новых событий на 80%.

---

## Дизайн

### Синтаксис

```rust
// Для обычных событий
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
    
    #[field(ty = "string", name = "prev_state")]
    previous_state: String,  // кастомное имя поля
}

// Для tracing_mark подтипов
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

// С несколькими темплейтами
#[trace_event(name = "sched_wakeup")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu} reason={reason}")]
#[derive(TraceEvent)]
struct TraceSchedWakeup {
    #[field(ty = "string")]
    comm: String,
    #[field(ty = "u32")]
    pid: u32,
    #[field(ty = "u32", optional)]
    reason: Option<u32>,
}

// payload_quick_check — явный impl при необходимости
impl FastMatch for TraceSchedWakeup {
    fn payload_quick_check(line: &str) -> bool {
        line.contains("reason=")
    }
}
```

### Атрибуты

| Атрибут | Назначение |
|---------|------------|
| `#[trace_event(name, aliases)]` | Имя события + алиасы |
| `#[trace_markers(...)]` | FastMatch PAYLOAD_MARKERS |
| `#[define_template("...")]` | Темплейт payload (можно несколько) |
| `#[derive(TraceEvent)]` | Генерация для обычных событий |
| `#[derive(TracingMarkEvent)]` | Генерация для tracing_mark |
| `#[field(ty, name, optional)]` | Атрибуты полей |

### Что генерирует `#[derive(TraceEvent)]`

1. `impl EventType` — `EVENT_NAME`, `EVENT_ALIASES`
2. `impl FastMatch` — `PAYLOAD_MARKERS` (из `#[trace_markers]`)
3. `impl TemplateEvent` — форматы (из `#[define_template]`)
4. `#[pymethods]` — конструктор, `__repr__`, `__eq__`, `can_be_parsed()`, `parse()`, `to_string()`, геттеры/сеттеры
5. Регистрация — `register_parser!(name, Struct)`

### Что генерирует `#[derive(TracingMarkEvent)]`

Всё то же самое +
- Регистрация — `register_tracing_mark_parser!(Struct)`

---

## Этапы реализации

### Этап 1: Настройка ✅
- [x] Создать ветку `feature/proc-macro`
- [x] Создать PROC_MACRO_TASK.md
- [ ] Создать `macros/` crate для proc-macro
- [ ] Настроить workspace в `Cargo.toml`

**Длительность:** 0.5 дня

### Этап 2: Парсинг атрибутов
- [ ] `#[trace_event(name, aliases)]` — парсинг имени и алиасов
- [ ] `#[trace_markers(...)]` — парсинг маркеров
- [ ] `#[define_template("...")]` — парсинг темплейтов
- [ ] `#[field(ty, name, optional)]` — парсинг атрибутов полей

**Длительность:** 1-2 дня

### Этап 3: Генерация traits
- [ ] `impl EventType` — EVENT_NAME, EVENT_ALIASES
- [ ] `impl FastMatch` — PAYLOAD_MARKERS
- [ ] `impl TemplateEvent` — форматы, parse_payload, render_payload

**Длительность:** 2-3 дня

### Этап 4: Python API
- [ ] Генерация `#[pymethods]`
- [ ] Конструктор с валидацией timestamp
- [ ] `__repr__`, `__eq__`, `__str__`
- [ ] `__copy__`, `__deepcopy__`
- [ ] `can_be_parsed()`, `parse()`, `to_string()`
- [ ] Геттеры/сеттеры для полей

**Длительность:** 2-3 дня

### Этап 5: Интеграция
- [ ] Регистрация через `register_parser!`
- [ ] Регистрация через `register_tracing_mark_parser!`
- [ ] Тесты интеграции с существующим кодом

**Длительность:** 1 день

### Этап 6: Тесты и документация
- [ ] Тесты макроса (snapshots)
- [ ] Примеры использования
- [ ] Документация в AGENTS.md, QWEN.md

**Длительность:** 1-2 дня

---

## Прогресс

| Этап | Статус | Дата начала | Дата завершения |
|------|--------|-------------|-----------------|
| 1. Настройка | 🔄 В процессе | | |
| 2. Парсинг атрибутов | ⏳ Ожидает | | |
| 3. Генерация traits | ⏳ Ожидает | | |
| 4. Python API | ⏳ Ожидает | | |
| 5. Интеграция | ⏳ Ожидает | | |
| 6. Тесты и документация | ⏳ Ожидает | | |

**Общий прогресс:** 0/6 этапов

---

## Заметки

### Зависимости для proc-macro

```toml
[dependencies]
proc-macro2 = "1.0"
quote = "1.0"
syn = { version = "2.0", features = ["full", "extra-traits"] }
```

### Структура macros crate

```
macros/
├── Cargo.toml
└── src/
    ├── lib.rs              # entry point
    ├── trace_event.rs      # #[derive(TraceEvent)]
    ├── tracing_mark_event.rs # #[derive(TracingMarkEvent)]
    ├── attrs.rs            # парсинг атрибутов
    └── utils.rs            # утилиты
```

### Известные проблемы

1. **Порядок итерации inventory** — не гарантирован, порядок парсинга tracing_mark определяется в `parse_tracing_mark()`
2. **Несколько темплейтов** — нужна автоматическая детекция формата по наличию полей
3. **Кастомные имена полей** — `#[field(name = "...")]` требует маппинга при парсинге и рендере
