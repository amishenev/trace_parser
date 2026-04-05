# trace_parser — Контекст для Qwen Code

## Обзор проекта

`trace_parser` — это библиотека на **Rust + PyO3** для парсинга больших текстовых логов `ftrace` / `tracefs`.

**Основная цель:** быстрый парсинг заголовков трассировки в Rust с поддержкой типизированных событий с плоской структурой полей.

### Ключевые возможности

- Парсинг общего формата строк трассировки в Rust
- Хранение данных в Rust до момента запроса из Python
- Поддержка типизированных классов событий
- Поддержка форматов полезной нагрузки для конкретных событий
- Семантический round-trip через `to_string()` / `from_string()`
- Поддержка service groups (принимаются при парсинге, но опускаются/нормализуются при выводе)
- Плоская структура полей — базовые поля доступны напрямую на каждом событии

### Поддерживаемые события

| Класс | Описание |
|-------|----------|
| `Trace` | Базовый класс для всех событий |
| `TraceSchedSwitch` | Переключение контекста планировщика |
| `TraceSchedWakeup` | Пробуждение процесса |
| `TraceSchedWakeupNew` | Пробуждение нового процесса |
| `TraceSchedProcessExit` | Завершение процесса |
| `TraceExit` | Завершение процесса (exit1, exit2) |
| `TraceCpuFrequency` | Изменение частоты CPU |
| `TraceDevFrequency` | Изменение частоты устройства (ddr_devfreq, l3c_devfreq) |
| `TracingMark` | Базовый класс для tracing_mark_write |
| `TraceMarkBegin` | Начало трассировочной метки |
| `TraceMarkEnd` | Конец трассировочной метки |
| `TraceReceiveVsync` | Специфичная метка ReceiveVsync |

## Формат строк трассировки

Базовый формат:

```text
TASK-TID (TGID) [CPU] FLAGS TIMESTAMP: event_name: payload
```

**Поля:**
- `thread_name` — имя потока
- `tid` — ID потока
- `tgid` — ID группы потоков
- `cpu` — ID процессора
- `flags` — флаги (например, `....`, `d..1`)
- `timestamp` — временная метка в секундах (float)
- `event_name` — имя события
- `payload_raw` — сырая полезная нагрузка

## Сборка и запуск

### Требования

- Python 3.10+
- Rust 1.80+ (текущая: 1.94)
- `uv` (менеджер пакетов)

### Установка окружения

```bash
# Создание виртуального окружения
uv venv .venv -p 3.10

# Активация окружения
source .venv/bin/activate

# Установка инструментов разработки
uv pip install maturin pytest

# Сборка расширения в Python-пакет
maturin develop
```

### Запуск тестов

```bash
# Rust тесты
cargo test -q

# Python smoke тесты
pytest -q tests/python

# Clippy (требуется в CI)
cargo clippy --all-targets -- -D warnings
```

### Пример использования

```python
from trace_parser import parse_trace, parse_trace_file, TraceSchedSwitch, TraceDevFrequency

# Парсинг через фабрику (однострочный)
event = parse_trace(line)

# Парсинг всего файла (быстрее чем построчный вызов parse_trace)
events = parse_trace_file("trace.txt")
events_filtered = parse_trace_file("trace.txt", filter_event="sched_switch")

# Прямой парсинг
switch = TraceSchedSwitch.parse(line)
if switch:
    print(switch.timestamp)
    print(switch.prev_comm, switch.next_comm)

# Частотные события
freq = TraceDevFrequency.parse(line)
if freq:
    print(freq.clk, freq.state)

# Доступ к payload и template
print(switch.payload)      # Отрендеренный payload
print(switch.template)     # Строка шаблона
```

## Архитектура проекта

### Структура каталогов

```
trace_parser/
├── src/                          # Rust исходники
│   ├── lib.rs                    # Точка входа, экспорты, parse_trace, parse_trace_file
│   ├── trace.rs                  # Базовый класс Trace
│   ├── common.rs                 # Общие утилиты парсинга, FastMatch, EventType
│   ├── payload_template.rs       # Шаблоны полезной нагрузки
│   ├── format_registry.rs        # Реестр форматов (multi-format)
│   ├── registry.rs               # Реестр парсеров событий
│   ├── sched_switch.rs           # sched_switch парсер (macro-generated)
│   ├── sched_wakeup.rs           # sched_wakeup/wakeup_new парсеры (macro-generated)
│   ├── sched_process_exit.rs     # sched_process_exit парсер
│   ├── frequency.rs              # Частотные события (macro-generated)
│   ├── trace_exit.rs             # exit1, exit2 парсеры
│   ├── tracing_mark/             # Tracing mark события
│   │   ├── base.rs               # TracingMark, TraceMarkBegin, TraceMarkEnd
│   │   └── receive_vsync.rs      # TraceReceiveVsync
│   └── tracing_mark_registry.rs  # Реестр tracing_mark подтипов
├── trace_parser/                 # Python пакет
│   ├── __init__.py               # Публичный API (реэкспорт из _native)
│   ├── __init__.pyi              # Type stubs
│   ├── _native.pyi               # Native module stubs
│   └── py.typed                  # PEP 561 marker
├── macros/                       # Proc-macro crate
│   ├── src/
│   │   ├── lib.rs                # trace_event_class, tracing_mark_event_class, TraceEvent, TracingMarkEvent, TraceEnum
│   │   ├── attrs.rs              # Парсинг атрибутов
│   │   ├── generator.rs          # Генерация trait impl
│   │   ├── pymethods.rs          # Генерация Python API
│   │   └── enum_gen.rs           # Генерация TraceEnum
│   └── examples/                 # Примеры использования макросов
├── tests/python/                 # Python smoke тесты
├── examples/                     # Примеры использования
└── benches/                      # Бенчмарки
```

### Архитектурные принципы

#### Плоская структура полей

Все базовые поля (`thread_name`, `tid`, `tgid`, `cpu`, `flags`, `timestamp`, `event_name`, `payload_raw`) объявлены напрямую в каждом типизированном классе:

```rust
pub struct TraceSchedSwitch {
    pub thread_name: String,
    pub tid: u32,
    pub tgid: u32,
    pub cpu: u32,
    pub flags: String,
    pub timestamp: f64,
    pub event_name: String,
    pub payload_raw: String,
    pub prev_comm: String,
    pub prev_pid: u32,
    // ...
}
```

**Доступ к полям в Python:**
- `sched_switch.timestamp` (прямой доступ)
- `sched_switch.prev_comm`

**Хелпер для извлечения базовых полей:**
```rust
// В src/trace.rs
fn extract_base_fields(captures: &Captures) -> Option<BaseFields>
```

#### Унифицированный payload/template API

Все события предоставляют:

- `payload` — getter для отрендеренной полезной нагрузки
- `template` — getter для строки шаблона

**Для простых событий** (`sched_*`, `frequency`):
- `payload()` возвращает `render_payload()` через шаблон
- `template()` возвращает строку шаблона

**Для `TracingMark`:**
- `payload` возвращает `&self.payload_raw`
- `template()` возвращает `"{payload}"`

**Для `TraceMarkBegin`/`TraceMarkEnd`/`TraceReceiveVsync`:**
- `payload` возвращает форматированную строку с `B|` или `E|` префиксом
- `message` getter для доступа к сообщению без префикса

#### format_trace_header helper

Общий хелпер для форматирования заголовка трассировки:

```rust
fn format_trace_header(
    thread_name: &str,
    tid: u32,
    tgid: u32,
    cpu: u32,
    flags: &str,
    timestamp: f64,
) -> String
```

**Формат вывода:**
```text
TASK-TID (TGID) [CPU] FLAGS TIMESTAMP:
```

#### PyO3 0.28 + Bound API

Проект использует PyO3 0.28 с новым `Bound` API:

```rust
use pyo3::prelude::*;
use pyo3::BoundObject;

// Возврат Python объектов
fn parse_trace(py: Python<'_>, line: &str) -> PyResult<Option<Py<PyAny>>> {
    // ...
}

// Хелпер для конвертации
fn parse_and_wrap<'py, T>(
    py: Python<'py>,
    line: &str,
    parser: fn(&str) -> Option<T>,
) -> Option<Py<PyAny>>
where
    T: IntoPyObject<'py>,
{
    parser(line)
        .and_then(|e| e.into_pyobject(py).ok())
        .map(|bound| bound.into_any().unbind())
}
```

**Ключевые изменения:**
- `IntoPy` → `IntoPyObject`
- `PyObject` → `Py<PyAny>`
- `into_py()` → `into_pyobject().into_any().unbind()`

#### std::sync::LazyLock

Rust 2024 использует `LazyLock` вместо `once_cell`:

```rust
use std::sync::LazyLock;

static FORMATS: LazyLock<FormatRegistry> = LazyLock::new(|| {
    FormatRegistry::new(vec![...])
});
```

#### FormatRegistry — словарь форматов

Для поддержки нескольких форматов payload используется `FormatRegistry`:

```rust
use crate::format_registry::{FormatRegistry, FormatSpec};
use std::sync::LazyLock;

static FORMATS: LazyLock<FormatRegistry> = LazyLock::new(|| {
    FormatRegistry::new(vec![
        FormatSpec { kind: "orig", template: &TEMPLATE_DEFAULT },
        FormatSpec { kind: "with_reason", template: &TEMPLATE_WITH_REASON },
    ])
});

// Для событий с одним форматом:
static FORMATS: LazyLock<FormatRegistry> = LazyLock::new(|| {
    FormatRegistry::single(&TEMPLATE)
});
```

**Детекция формата:**
```rust
impl TemplateEvent for TraceSchedWakeup {
    fn formats() -> &'static FormatRegistry { &FORMATS }
    
    fn detect_format(payload: &str) -> &'static str {
        if payload.contains("reason=") { "with_reason" } else { "orig" }
    }
    // ...
}
```

#### Шаблонизация полезной нагрузки

Простые форматы payload описываются через `PayloadTemplate`:

```rust
PayloadTemplate::new(
    "prev_comm={prev_comm} prev_pid={prev_pid} ==> next_comm={next_comm}",
    &[
        ("prev_comm", FieldSpec::string()),
        ("prev_pid", FieldSpec::u32()),
        ("next_comm", FieldSpec::string()),
    ]
)
```

**Поддерживаемые FieldSpec:**
- `FieldSpec::string()` — строка
- `FieldSpec::u32()` — беззнаковое 32-bit
- `FieldSpec::i32()` — знаковое 32-bit
- `FieldSpec::f64()` — float
- `FieldSpec::bool_int()` — булево как 0/1
- `FieldSpec::choice(&[...])` — выбор из списка

**Service tokens:**
- `{ws}` — `\s+`, рендерится как пробел
- `{?ws}` — `\s*`, рендерится как пустота
- `{ignore:name}` — игнорируемое поле
- `{?ignore:name}` — опциональное игнорируемое поле

#### Fast-match эвристики

Для оптимизации используется двухуровневая проверка:

1. `FastMatch::quick_check` — быстрая проверка через:
   - `extract_event_name()` (SIMD memchr) для event_name
   - `PAYLOAD_MARKERS` (SIMD memchr) для payload маркеров
   - `payload_quick_check()` для кастомной сложной логики

2. Полноценный regex-парсинг только после успешной быстрой проверки

**Примеры PAYLOAD_MARKERS:**
- `TraceReceiveVsync`: `&[b"B|", b"ReceiveVsync"]`
- `TraceMarkBegin`: `&[b"B|"]`
- `TraceMarkEnd`: `&[b"E|"]`

**tracing_mark_registry:** отдельный реестр для подтипов tracing_mark_write с явным порядком парсинга.

## Конвенции разработки

### Commit messages

Использовать **Conventional Commits** на английском:

```
feat: add sched_waking parser
fix: separate pyo3 extension-module feature for CI
docs: update README with release workflow
ci: expand Python version matrix
```

### Структура Rust файлов

1. `#[pyclass]` определение класса
2. `impl Type` — вспомогательные методы
3. Trait implementations (`EventType`, `FastMatch`, `TemplateEvent`)
4. `#[pymethods] impl Type` — Python API
5. Тесты в конце файла

### Python workflow

- Использовать `uv` для всех Python операций
- Виртуальное окружение: `.venv`
- Python 3.10 — минимальная версия
- `pyo3` использует `abi3-py310` (версия 0.28.2+)
- Rust edition 2024
- Native extension: `trace_parser._native`

**Артефакты сборки:**
- `_native.abi3.so` под `python/trace_parser/`
- macOS: `*.dSYM/` директории
- Игнорируются git, не копировать вручную

### Добавление новых событий

Для каждого нового публичного события:

1. Экспортировать класс из Rust модуля
2. Обновить `python/trace_parser/__init__.py`
3. Добавить/обновить `.pyi` файл
4. Добавить Python пример в `examples/`
5. Добавить smoke тест в `tests/python/`

**Важно:** При изменении публичного API (добавление/удаление полей, свойств, методов, изменение структуры классов) обязательно проверяйте соответствие type stubs (`.pyi` файлов) новой версии API. Стабы должны быть синхронизированы с Rust реализацией.

## CI/CD

### GitHub Actions

**CI (ci.yml):**
- Тестирование на Ubuntu и macOS
- Python версии: 3.10, 3.11, 3.12, 3.13, 3.14
- Python 3.15-dev — allowed-to-fail

**Release (release.yml):**
- Триггер: теги вида `v0.1.0`
- Сборка wheel для Linux, macOS, Windows
- Сборка sdist
- Публикация в GitHub Releases

**Commitlint:**
- Проверка формата commit messages

**Coverage (coverage.yml):**
- Генерация отчёта покрытия через cargo-llvm-cov
- Загрузка HTML артефакта
- Отправка отчёта в Codacy

### Настройка Codacy

1. Зарегистрироваться на https://app.codacy.com
2. Добавить репозиторий
3. Получить Project Token в Settings → Coverage
4. Добавить GitHub Secret `CODACY_PROJECT_TOKEN`

### Пример релиза

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Важные детали реализации

### PyO3 0.28 миграция

**Текущая версия:** PyO3 0.28, Rust 2024

**Ключевые изменения:**
- `IntoPy<T>` → `IntoPyObject<'py>`
- `PyObject` → `Py<PyAny>`
- `into_py(py)` → `into_pyobject(py)?.into_any().unbind()`
- `once_cell::sync::Lazy` → `std::sync::LazyLock`

**Хелпер для конвертации:**
```rust
fn parse_and_wrap<'py, T>(
    py: Python<'py>,
    line: &str,
    parser: fn(&str) -> Option<T>,
) -> Option<Py<PyAny>>
where
    T: IntoPyObject<'py>,
{
    parser(line)
        .and_then(|e| e.into_pyobject(py).ok())
        .map(|bound| bound.into_any().unbind())
}
```

### SIMD оптимизации

**Зависимости:**
- `memchr = "2.7"` — SIMD поиск подстроки
- `lexical-core = "1.0"` — SIMD парсинг чисел

**Примеры:**
```rust
use memchr::memmem;
use lexical_core::parse;

// Поиск подстроки
let pos = memmem::find(line.as_bytes(), b": ")?;

// Парсинг чисел
let tid: u32 = parse(captures.name("tid")?.as_str().as_bytes()).ok()?;
let timestamp: f64 = parse(captures.name("timestamp")?.as_str().as_bytes()).ok()?;
```

**Где используется:**
- `extract_event_name()` — memchr для поиска `": "`
- `BaseTraceParts::parse()` — lexical-core для tid, tgid, cpu, timestamp
- `cap_parse()` — универсальная функция через `FromLexical`

### Парсинг timestamp

- Хранится в секундах как `f64`
- Геттеры/сеттеры для `timestamp_ms` и `timestamp_ns`
- Валидация через `validate_timestamp()`

### BoolInt поля

- В тексте: `0` / `1`
- В Rust/Python модели: `bool`
- При рендере обратно в `0` / `1`

### Иерархия tracing_mark

Все события имеют `event_name == "tracing_mark_write"`:

```
TracingMark (базовый)
├── TraceMarkBegin (B|tgid|payload)
├── TraceMarkEnd (E|tgid|payload)
└── TraceReceiveVsync (специфичный Begin)
```

**Порядок парсинга (tracing_mark_registry):**
1. Зарегистрированные специфичные подтипы (ReceiveVsync, RequestVsync, SubmitVsync...)
2. TraceMarkBegin (захардкожено)
3. TraceMarkEnd (захардкожено)
4. TracingMark (fallback)

## Отложенные улучшения

### Решённые улучшения

**Плоская структура полей** — реализована в последних изменениях:

- Все базовые поля объявлены напрямую в каждом классе
- Прямой доступ: `event.timestamp` вместо `event.base.timestamp`
- `TraceReceiveVsync`: было 4 уровня вложенности, стал 1 уровень
- Быстрее доступ, меньше накладных расходов

### Отложенные улучшения

**Proc-macro генерация pymethods** — ✅ завершена:

**Реализованные фичи:**

| Фича | Синтаксис | Статус |
|------|-----------|--------|
| Вывод типов | `#[field]` без `ty` | ✅ |
| Кастомный regex | `#[field(regex = r"\d{3}")]` | ✅ |
| Choice | `#[field(choice = ["a", "b"])]` | ✅ |
| Enum | `#[derive(TraceEnum)]` + `#[value("...")]` | ✅ |
| `generate_pymethods` | `generate_pymethods = true/false` | ✅ |
| Формат рендера | `#[field(format = "{:03}")]` | ✅ |
| Multi-template | несколько `#[define_template(...)]` + `detect = [...]` | ✅ |
| Fast-match | `#[fast_match(contains_any = [...])]` | ✅ |
| TracingMark | `#[tracing_mark_event_class]` + `begin`/`end` | ✅ |

**Что работает:**
- `macros/` crate с `#[trace_event_class]`, `#[tracing_mark_event_class]`, `#[derive(TraceEvent)]`, `#[derive(TracingMarkEvent)]`, `#[derive(TraceEnum)]`
- Генерация `EventType`, `FastMatch`, `TemplateEvent`
- Генерация `#[pymethods]` с `new`, `can_be_parsed`, `parse`, `to_string`, `payload`, `template`
- Type inference из Rust-типа (String, u8/u16/u32/u64, i8/i16/i32/i64, f32/f64, bool, Option<T>)
- Кастомный regex для полей с нестандартным форматом
- Choice для полей с ограниченным набором значений
- `#[field(format = "...")]` для кастомного рендера (например `{:03}` → `000`)
- Multi-template с SIMD-детекцией формата через `detect = ["..."]`
- `#[derive(TraceEnum)]` — генерация Display, FromStr, TraceEnum trait
- `skip_registration` для TraceMarkBegin/End (регистрируются явно)
- `Option<T>` для опциональных полей (без `#[field(optional)]`)
- Python-доступ по `#[field(...)]`: `readonly`/`private`/обычное поле

**Все типовые события на макросах:** `TraceSchedSwitch`, `TraceSchedWakeup`, `TraceSchedWakeupNew`, `TraceSchedProcessExit`, `TraceExit`, `TraceCpuFrequency`, `TraceDevFrequency`, `TracingMark`, `TraceMarkBegin`, `TraceMarkEnd`, `TraceReceiveVsync`

**Единственное ручное событие:** `Trace` (базовый fallback класс, не `TemplateEvent`)

**Что остаётся (не связано с миграцией):**
- Наследование через PyO3 `extends` — см. INHERITANCE_PLAN.md
- E2E интеграционные тесты
- Генерация Python type stubs (.pyi) из макросов

## Ссылки

- [README.md](README.md) — основная документация
- [AGENTS.md](AGENTS.md) — подробные заметки для разработчиков
- [examples/basic_usage.py](examples/basic_usage.py) — примеры использования
- [tests/python/](tests/python/) — smoke тесты
