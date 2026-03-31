# trace_parser_macros — Заметки для разработчиков

## Архитектура

```
macros/src/
├── lib.rs           # Entry point, derive макросы
├── attrs.rs         # Парсинг атрибутов
├── generator.rs     # Генерация кода (traits)
└── pymethods.rs     # Генерация Python API
```

## Модули

### `lib.rs`

- `derive_trace_event` — основной макрос для обычных событий
- `derive_tracing_mark_event` — макрос для tracing_mark подтипов
- Парсит атрибуты, вызывает генераторы

### `attrs.rs`

Парсинг атрибутов:
- `TraceEventAttr` — `#[trace_event(name, aliases)]`
- `TraceMarkersAttr` — `#[trace_markers(...)]`
- `DefineTemplateAttr` — `#[define_template("...")]`
- `FieldAttr` — `#[field(ty, name, optional, readonly, private)]`

### `generator.rs`

Генерация impl блоков:
- `generate_event_type_impl` — `impl EventType`
- `generate_fast_match_impl` — `impl FastMatch`
- `generate_template_event_impl` — `impl TemplateEvent`
- `generate_registration` — регистрация через `register_parser!`
- `generate_tracing_mark_registration` — регистрация через `register_tracing_mark_parser!`

### `pymethods.rs`

Генерация Python API:
- `generate_pymethods_block` — основной блок `#[pymethods]`
- `generate_new` — конструктор
- `generate_repr`, `generate_eq`, `generate_str` — repr/eq/str
- `generate_can_be_parsed`, `generate_parse`, `generate_to_string` — парсинг
- `generate_copy`, `generate_deepcopy` — копирование
- `generate_payload`, `generate_template` — геттеры
- `generate_field_accessors` — геттеры/сеттеры для полей

---

## Known Issues

### 1. `field_accessors` не используется

**Проблема:** В `generate_pymethods_block` генерируется `field_accessors`, но не вставляется в output.

**Решение:** Нужно либо использовать, либо удалить.

### 2. `render_payload` и `parse_payload` — заглушки

**Проблема:** Генерируются `Ok(String::new())` и `None`.

**Решение:** Нужно генерировать код на основе `field_specs`.

### 3. Базовые поля в конструкторе

**Проблема:** Конструктор требует все базовые поля (`thread_name`, `thread_tid`, etc.) как параметры.

**Решение:** Это правильное поведение — пользователь должен передавать их при создании события.

### 4. `struct_name` не используется в некоторых функциях

**Проблема:** Функции принимают `struct_name: &Ident`, но не используют.

**Решение:** Убрать параметр или использовать.

---

## План развития

### Краткосрочный (1-2 недели)

- [ ] Исправить `render_payload` — генерация на основе field_specs
- [ ] Исправить `parse_payload` — парсинг на основе field_specs
- [ ] Использовать `field_accessors` в output
- [ ] Убрать неиспользуемые параметры

### Среднесрочный (1 месяц)

- [ ] Автоматическая детекция формата по наличию полей
- [ ] Поддержка `#[field(choice = ["A", "B", "C"])]`
- [ ] Поддержка `#[field(pattern = r"\d+")]` для кастомных regex

### Долгосрочный (2-3 месяца)

- [ ] Макрос для автоматической генерации `#[define_template]` из полей
- [ ] Поддержка вложенных структур
- [ ] Генерация Python type stubs (.pyi)

---

## Тестовые сценарии

### Что тестировать

1. **Парсинг атрибутов**
   - `#[trace_event(name = "...")]` — обязательный
   - `#[trace_event(name = "...", aliases = ["..."])]` — с алиасами
   - `#[field(ty = "string")]` — базовый тип
   - `#[field(ty = "u32", name = "...")]` — кастомное имя
   - `#[field(optional)]` — опциональное поле
   - `#[field(readonly)]` — readonly
   - `#[field(private)]` — private

2. **Генерация кода**
   - `impl EventType` — правильное имя и алиасы
   - `impl FastMatch` — маркеры или пустой impl
   - `impl TemplateEvent` — форматы, parse_payload, render_payload
   - `#[pymethods]` — конструктор, методы, геттеры

3. **Регистрация**
   - `register_parser!` — для обычных событий
   - `register_tracing_mark_parser!` — для tracing_mark

4. **Python API**
   - Конструктор с базовыми полями
   - `can_be_parsed()`, `parse()`, `to_string()`
   - `payload`, `template` геттеры
   - Геттеры/сеттеры для полей

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
fn test_tracing_mark_event() {
    // Проверяем регистрацию tracing_mark событий
}
```

---

## Интеграция с основным crate

Макросы используют пути к основному crate:
- `::trace_parser::common::EventType`
- `::trace_parser::register_parser!`
- `::trace_parser::payload_template::PayloadTemplate`

**Важно:** Основной crate должен экспортировать эти items.
