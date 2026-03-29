# SIMD Оптимизации — Результаты бенчмарков

## Python бенчмарки (pytest-benchmark)

**Платформа:** macOS (Darwin-CPython-64bit)  
**Файл:** 1,000 строк

### SIMD vs Без SIMD (Python 3.14)

| Тест | SIMD | Без SIMD | Ускорение |
|------|------|----------|-----------|
| test_python_with_filter | **4.4ms** | 33.2ms | **7.5x быстрее** |
| test_python_line_by_line | **15.7ms** | 145.0ms | **9.2x быстрее** |
| test_rust_parse_file_with_filter | **48.8ms** | 478.5ms | **9.8x быстрее** |
| test_rust_parse_file | **144.6ms** | 1355.3ms | **9.4x быстрее** |

### Python 3.10 (с SIMD)

| Тест | Время | Относительно |
|------|-------|--------------|
| test_python_with_filter | 4.7ms | 1.0x |
| test_python_line_by_line | 18.9ms | 4.1x медленнее |
| test_rust_parse_file_with_filter | 51.5ms | 11.1x медленнее |
| test_rust_parse_file | 164.3ms | 35.3x медленнее |

---

## Rust бенчмарки (cargo bench)

**Дата:** 29 марта 2026  
**Платформа:** macOS (Darwin-CPython-3.14-64bit)  
**Итераций:** 300,000

### Positive case (sched_switch)

#### Fast checks (SIMD vs scalar)

| Метод | Время | Ускорение |
|-------|-------|-----------|
| `contains() [SIMD memchr]` | **14.1 ns/op** | **9.7x** vs scalar |
| `contains() [scalar find()]` | 137.1 ns/op | baseline |
| `contains_shape() [SIMD memchr]` | **93.9 ns/op** | **8.3x** vs scalar |
| `contains_shape() [scalar find()]` | 776.7 ns/op | baseline |

#### Full parse (typed events)

| Метод | Время | Ускорение |
|-------|-------|-----------|
| `TraceSchedSwitch::can_be_parsed() [SIMD]` | **87.6 ns/op** | **104x** vs parse |
| `Trace::can_be_parsed() [SIMD]` | **304.2 ns/op** | **22x** vs parse |
| `Trace::parse() [regex]` | 6612.3 ns/op | baseline |
| `TraceSchedSwitch::parse() [regex]` | 9076.2 ns/op | baseline |

### Negative case

#### Fast checks (SIMD vs scalar)

| Метод | Время | Ускорение |
|-------|-------|-----------|
| `contains() [SIMD memchr]` | **15.5 ns/op** | **6.2x** vs scalar |
| `contains() [scalar find()]` | 95.8 ns/op | baseline |
| `contains_shape() [SIMD memchr]` | **15.6 ns/op** | **6.3x** vs scalar |
| `contains_shape() [scalar find()]` | 97.5 ns/op | baseline |

#### Full parse (typed events)

| Метод | Время |
|-------|-------|
| `TraceSchedSwitch::parse() [regex]` | 84.0 ns/op |
| `TraceSchedSwitch::can_be_parsed() [SIMD]` | 108.7 ns/op |
| `Trace::can_be_parsed() [SIMD]` | 195.2 ns/op |
| `Trace::parse() [regex]` | 4788.3 ns/op |

---

## Python бенчмарки (pytest-benchmark)

**Дата:** 29 марта 2026  
**Платформа:** macOS  
**Файл:** 10,000 строк

### Python 3.14

| Тест | Время | Относительно |
|------|-------|--------------|
| test_python_with_filter | 50ms | 1.0x |
| test_python_line_by_line | 139ms | 2.8x медленнее |
| test_rust_parse_file_with_filter | 495ms | 9.9x медленнее |
| test_rust_parse_file | 1598ms | 31.8x медленнее |

**Примечание:** Rust parse_trace_file() медленнее из-за FFI overhead при создании PyObject для каждого события.

---

## Выводы

### SIMD оптимизации (memchr + lexical-core)

1. **`contains()` через memchr быстрее scalar `find()` в 6-10 раз**
2. **`Trace::can_be_parsed()` с SIMD в 22x быстрее** чем полный regex парсинг
3. **`TraceSchedSwitch::can_be_parsed()` с SIMD в 104x быстрее** чем полный парсинг

### FFI overhead

Python бенчмарки показывают что FFI overhead доминирует:
- Rust парсинг: ~300 ns/op (SIMD) vs ~6600 ns/op (regex)
- Python + FFI: ~50-1600 ms/op из-за создания PyObject

### Рекомендации

1. **Использовать `can_be_parsed()` для быстрой фильтрации** перед парсингом
2. **Избегать FFI в горячих циклах** — группировать вызовы
3. **Для bulk парсинга** использовать `parse_trace_file()` (один FFI вызов)

---

## Изменения в коде

### Файлы с SIMD оптимизациями

1. `src/lib.rs` — `extract_event_name()` с memchr
2. `src/common.rs` — `contains_event_name()` и `BaseTraceParts::parse()` с lexical-core
3. `src/tracing_mark/receive_vsync.rs` — парсинг frame_number с lexical-core
4. `benches/can_be_parsed.rs` — бенчмарки SIMD vs scalar

### Зависимости

```toml
[dependencies]
memchr = "2.7"        # SIMD поиск подстроки
lexical-core = "1.0"  # SIMD парсинг чисел
```
