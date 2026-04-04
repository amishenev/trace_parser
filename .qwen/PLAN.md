# План доработки макросов

## 1. Форматирование полей (format атрибут)

**Цель:** универсальный механизм кастомного формата при рендере payload.

**Задача:** заменить `zero_pad` (удалён) на `#[field(format = "{:03}")]`.

**Файлы:**
- `macros/src/attrs.rs` — добавить `format: Option<String>` в `FieldAttr`
- `macros/src/generator.rs` — в render_statements: если есть `format` → `TemplateValue::Str(&format!(FORMAT, value))`
- `macros/QWEN.md` — документация
- Тесты — парсинг `format` атрибута

**Пример использования:**
```rust
#[field(regex = r"\d{3}", format = "{:03}")]
pub target_cpu: u32,
```

---

## 2. Автоматическая регистрация tracing mark событий

**Цель:** вернуть `register_tracing_mark` (удалён в предыдущем коммите).

**Задача:** TracingMarkEvent должны опционально регистрироваться в `tracing_mark_registry`.

**Файлы:**
- `macros/src/attrs.rs` — вернуть `register_tracing_mark: bool` (default true)
- `macros/src/generator.rs` — вернуть `generate_tracing_mark_registration`
- `macros/src/lib.rs` — conditional registration в derive
- Тесты — проверка генерации

---

## 3. Детекция форматов (format markers)

**Цель:** механизм для `detect_format` — аналог `PAYLOAD_MARKERS` но для шаблонов.

**Задача:** сейчас `detect_format` — заглушка `0`. Позже добавить массив подстрок-маркеров + fallback.

**Идея (не реализована):**
```rust
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}")]
#[define_template("comm={comm} pid={pid} prio={prio} target_cpu={target_cpu} reason={reason}", format_markers = ["reason="])]
```

Или на уровне события:
```rust
#[trace_event(name = "sched_wakeup", format_markers = ["reason="])]
```

**Статус:** отложено, пока не нужен план реализации.
