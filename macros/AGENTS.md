# trace_parser_macros — Заметки для разработчиков

## Архитектура

```
macros/src/
├── lib.rs           # Entry point, derive макросы
├── attrs.rs         # Парсинг атрибутов
├── generator.rs     # Генерация кода (traits)
├── pymethods.rs     # Генерация Python API
└── enum_gen.rs      # Генерация TraceEnum (Display, FromStr)
```

## Модули

### `lib.rs`

- `derive_trace_event` — основной макрос для обычных событий
- `derive_tracing_mark_event` — макрос для tracing_mark подтипов
- `derive_trace_enum` — макрос для enum-типов (Display, FromStr, TraceEnum)
- Парсит атрибуты, вызывает генераторы

### `attrs.rs`

Парсинг атрибутов:
- `TraceEventAttr` — `#[trace_event(name, aliases, skip_registration, begin, end)]`
- `MarkType` — `Begin` / `End` для TracingMarkEvent
- `FastMatchAttr` — `#[fast_match(contains_any = [...])]`
- `TraceMarkersAttr` — `#[trace_markers(...)]`
- `DefineTemplateAttr` — `#[define_template("...")]`
- `FieldAttr` — `#[field(name, regex, choice, format, optional, readonly, private)]`

### `generator.rs`

Генерация impl блоков:
- `generate_event_type_impl` — `impl EventType`
- `generate_fast_match_impl` — `impl FastMatch`
- `generate_template_event_impl` — `impl TemplateEvent`
- `generate_registration` — регистрация через `register_parser!` или `register_tracing_mark_parser!`
- `InferredType` — вывод типов из Rust-типа

### `pymethods.rs`

Генерация Python API:
- `generate_pymethods_block` — основной блок `#[pymethods]`
- `generate_new` — конструктор
- `generate_repr`, `generate_eq`, `generate_str` — repr/eq/str
- `generate_can_be_parsed`, `generate_parse`, `generate_to_string` — парсинг
- `generate_copy`, `generate_deepcopy` — копирование
- `generate_payload`, `generate_template` — геттеры
- `generate_field_accessors` — геттеры/сеттеры для полей

### `enum_gen.rs`

Генерация для `#[derive(TraceEnum)]`:
- `generate_trace_enum` — Display, FromStr, TraceEnum trait
- `parse_variants` — парсинг `#[value("...")]` атрибутов

---

## Known Issues

**Все основные проблемы исправлены в этапах 1-6.**

### Отложено

- **E2E тесты с реальным событием** — требуют дополнительной проработки архитектуры макроса
- **`#[pyclass]`** — должен указываться пользователем вручную
- **Миграция всех событий на макрос** — только TraceSchedSwitch использует сейчас
- **Наследование через PyO3 `extends`** — см. INHERITANCE_PLAN.md

---

## План развития

### Краткосрочный (1-2 недели)

- [x] Исправить `render_payload` — генерация на основе field_specs ✅
- [x] Исправить `parse_payload` — парсинг на основе field_specs ✅
- [x] Использовать `field_accessors` в output ✅
- [x] Убрать неиспользуемые параметры ✅

### Среднесрочный (1 месяц)

- [x] Автоматическая детекция формата по наличию полей ✅
- [x] Поддержка `#[field(choice = ["A", "B", "C"])]` ✅
- [x] Поддержка `#[field(regex = r"\d+")]` для кастомных regex ✅
- [x] Вывод типов из Rust-типа (`#[field]` без `ty`) ✅

### Долгосрочный (2-3 месяца)

- [ ] Наследование через PyO3 `extends` — см. INHERITANCE_PLAN.md
- [ ] Миграция всех событий на макрос
- [ ] E2E интеграционные тесты
- [ ] Макрос для автоматической генерации `#[define_template]` из полей
- [ ] Поддержка вложенных структур
- [ ] Генерация Python type stubs (.pyi)
- [ ] Автоматическое добавление `#[pyclass]`

---

## Тестовые сценарии

### Что тестировать

1. **Парсинг атрибутов**
   - `#[trace_event(name = "...")]` — обязательный
   - `#[trace_event(name = "...", aliases = ["..."])]` — с алиасами
   - `#[field]` — без аргументов, вывод типа
   - `#[field(name = "...")]` — кастомное имя
   - `#[field(regex = "...")]` — кастомный regex
   - `#[field(choice = [...])]` — ограниченный набор
   - `#[field(optional)]` — опциональное поле
   - `#[field(readonly)]` — readonly
   - `#[field(private)]` — private

2. **Генерация кода**
   - `impl EventType` — правильное имя и алиасы
   - `impl FastMatch` — маркеры или пустой impl
   - `impl TemplateEvent` — форматы, parse_payload, render_payload
   - `#[pymethods]` — конструктор, методы, геттеры

3. **TraceEnum генерация**
   - Display — правильное строковое представление
   - FromStr — парсинг обратно в enum
   - TraceEnum trait — values() метод

### TracingMarkEvent derive

Специальная обработка для `tracing_mark_write` событий:

1. **Флаги `begin`/`end`** — определяют тип маркера
2. **Авто-префикс шаблонов** — `B|{trace_mark_tgid}|` или `E|{trace_mark_tgid}|`
3. **Объединение маркеров** — `["B|", ...user_markers]`
4. **Регистрация** — `TracingMarkEntry` или пропуск через `skip_registration`

```rust
// Begin: маркеры ["B|"], шаблон "B|{trace_mark_tgid}|{message}"
#[trace_event(name = "tracing_mark_write", begin, skip_registration)]
#[define_template("{message}")]

// End: маркеры ["E|"], шаблон "E|{trace_mark_tgid}|{message}"
#[trace_event(name = "tracing_mark_write", end, skip_registration)]
#[define_template("{message}")]

// Специфичный: маркеры ["B|", "ReceiveVsync"], шаблон "B|{trace_mark_tgid}|..."
#[trace_event(name = "tracing_mark_write", begin)]
#[trace_markers("ReceiveVsync")]
#[define_template("{message}")]
```

### Регистрация
Макрос генерирует `inventory::submit!` напрямую — обёрточные макросы `register_parser!` и `register_tracing_mark_parser!` удалены.

### Примеры тестов

```rust
#[test]
fn test_basic_event() {
    // Проверяем генерацию для простого события
}

#[test]
fn test_custom_field_name() {
    // Проверяем маппинг имён полей
}

#[test]
fn test_optional_field() {
    // Проверяем Option<T> для опциональных полей
}

#[test]
fn test_custom_regex() {
    // Проверяем кастомный regex
}

#[test]
fn test_choice_field() {
    // Проверяем FieldSpec::choice
}

#[test]
fn test_trace_enum() {
    // Проверяем генерацию Display/FromStr/TraceEnum
}
```

---

## Интеграция с основным crate

Макросы используют пути к основному crate:
- `::trace_parser::common::EventType`
- `::trace_parser::register_parser!`
- `::trace_parser::payload_template::PayloadTemplate`
- `::trace_parser::payload_template::TraceEnum`

**Важно:** Основной crate должен экспортировать эти items.
