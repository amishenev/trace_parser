# trace_parser — Контекст для Qwen Code

## Обзор проекта

`trace_parser` — это библиотека на **Rust + PyO3** для парсинга больших текстовых логов `ftrace` / `tracefs`.

**Основная цель:** быстрый парсинг заголовков трассировки в Rust с поддержкой типизированных событий поверх универсального базового класса `Trace`.

### Ключевые возможности

- Парсинг общего формата строк трассировки в Rust
- Хранение данных в Rust до момента запроса из Python
- Поддержка типизированных классов событий
- Поддержка форматов полезной нагрузки для конкретных событий
- Семантический round-trip через `to_string()` / `from_string()`
- Поддержка service groups (принимаются при парсинге, но опускаются/нормализуются при выводе)

### Поддерживаемые события

| Класс | Описание |
|-------|----------|
| `Trace` | Базовый класс для всех событий |
| `TraceSchedSwitch` | Переключение контекста планировщика |
| `TraceSchedWakeup` | Пробуждение процесса |
| `TraceSchedWakeupNew` | Пробуждение нового процесса |
| `TraceSchedProcessExit` | Завершение процесса |
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
```

### Пример использования

```python
from trace_parser import parse_trace, TraceSchedSwitch, TraceDevFrequency

# Парсинг через фабрику
event = parse_trace(line)

# Прямой парсинг
switch = TraceSchedSwitch.parse(line)
if switch:
    print(switch.base.timestamp)
    print(switch.prev_comm, switch.next_comm)

# Частотные события
freq = TraceDevFrequency.parse(line)
if freq:
    print(freq.clk, freq.state)
```

## Архитектура проекта

### Структура каталогов

```
trace_parser/
├── src/                          # Rust исходники
│   ├── lib.rs                    # Точка входа, экспорты
│   ├── trace.rs                  # Базовый класс Trace
│   ├── common.rs                 # Общие утилиты парсинга
│   ├── payload_template.rs       # Шаблоны полезной нагрузки
│   ├── sched_switch.rs           # sched_switch парсер
│   ├── sched_wakeup.rs           # sched_wakeup/wakeup_new парсеры
│   ├── sched_process_exit.rs     # sched_process_exit парсер
│   ├── frequency.rs              # Частотные события
│   └── tracing_mark/             # Tracing mark события
│       ├── base.rs               # TracingMark, TraceMarkBegin, TraceMarkEnd
│       └── receive_vsync.rs      # TraceReceiveVsync
├── python/trace_parser/          # Python обёртки
│   ├── __init__.py               # Публичный API
│   ├── __init__.pyi              # Type stubs
│   ├── trace.py(i)               # Trace класс
│   ├── sched_switch.py(i)        # TraceSchedSwitch
│   ├── sched_wakeup.py(i)        # TraceSchedWakeup/New
│   ├── sched_process_exit.py(i)  # TraceSchedProcessExit
│   ├── frequency.py(i)           # TraceCpu/DevFrequency
│   └── tracing_mark/             # Tracing mark модули
│       ├── base.py(i)
│       └── receive_vsync.py(i)
├── tests/python/                 # Python smoke тесты
├── examples/                     # Примеры использования
└── benches/                      # Бенчмарки
```

### Архитектурные принципы

#### Композиция вместо наследования

Типизированные классы используют композицию:

```rust
pub struct TraceSchedSwitch {
    pub base: Trace,           // Базовые поля через вложенность
    pub prev_comm: String,
    pub prev_pid: u32,
    // ...
}
```

**Доступ к полям в Python:**
- `sched_switch.base.timestamp`
- `sched_switch.prev_comm`

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

1. `FastMatch::quick_check` — быстрая проверка по `event_name`
2. Полноценный regex-парсинг только после успешной быстрой проверки

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

## Отложенные улучшения

### Python ergonomics

Текущий доступ: `vsync.begin.mark.base.timestamp`

Желаемый доступ: `vsync.timestamp`

**Решение отложено** до выбора единого механизма proxy-полей для всех событий.

## Ссылки

- [README.md](README.md) — основная документация
- [AGENTS.md](AGENTS.md) — подробные заметки для разработчиков
- [examples/basic_usage.py](examples/basic_usage.py) — примеры использования
- [tests/python/](tests/python/) — smoke тесты
