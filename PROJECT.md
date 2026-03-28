# trace_parser

## Что это

`trace_parser` это библиотека на `Rust + PyO3` для быстрого разбора текстовых `ftrace` / `tracefs` строк формата:

```text
TASK-TID (TGID) [CPU] FLAGS TIMESTAMP: event_name: payload
```

Текущая цель проекта:

- быстро разбирать базовую часть trace-строки в Rust
- держать данные в Rust до явного запроса
- поддерживать typed-классы поверх общего `Trace`
- для простых payload-форматов описывать parse + format через один шаблон

## Текущее устройство

### Базовый слой

- `src/trace.rs`
  - класс `Trace`
  - умеет:
    - `Trace.can_be_parsed(line)`
    - `Trace.parse(line)`
    - `to_string()`
  - хранит:
    - `thread_name`
    - `tid`
    - `tgid`
    - `cpu`
    - `flags`
    - `timestamp`
    - `event_name`
    - `payload_raw`

- `src/common.rs`
  - общий regex базовой trace-строки
  - разбор `BaseTraceParts`
  - валидация timestamp

### Typed events

- `src/sched_switch.rs`
  - класс `TraceSchedSwitch`
  - использует композицию: `base: Trace`
  - собственные payload-поля лежат отдельно от `base`

- `src/tracing_mark.rs`
  - `TracingMark`
  - `TraceMarkBegin`
  - `TraceMarkEnd`
  - `TraceReceiveVsync`

### Шаблоны payload

- `src/payload_template.rs`
  - `PayloadTemplate`
  - `FieldSpec`
  - `TemplateValue`

Этот слой нужен, чтобы один раз задать строку формата payload и использовать ее:

- для генерации regex
- для `to_string()`

## Правила описания payload

### Обычные поля

Формат задается строкой с именованными группами:

```text
prev_comm={prev_comm} prev_pid={prev_pid} ==> next_comm={next_comm} next_pid={next_pid}
```

Типы полей задаются отдельно, массивом пар:

```rust
&[
    ("prev_comm", FieldSpec::string()),
    ("prev_pid", FieldSpec::u32()),
    ("next_comm", FieldSpec::string()),
    ("next_pid", FieldSpec::u32()),
]
```

Это сделано специально:

- строка формата остается короткой и читаемой
- типы полей не зашиваются в сам шаблон
- новые типы добавляются через `FieldSpec`

### Сервисные группы

Служебные части payload не должны засорять typed-модель.

Примеры:

- лишние пробелы
- префиксы вроде `[ExtraInfo]`
- другие артефакты, которые надо принять на parse, но выбросить при `to_string()`

Сейчас поддержаны встроенные сервисные токены шаблона:

- `{ws}`: матчится как `\s+`, в `to_string()` печатается как один пробел
- `{?ws}`: матчится как `\s*`, в `to_string()` исчезает

## Фабричный parse

В `src/lib.rs` есть фабричная функция:

```python
trace_parser.parse_trace(line)
```

Она пытается вернуть самый конкретный класс из известных:

1. `TraceReceiveVsync`
2. `TraceMarkBegin`
3. `TraceMarkEnd`
4. `TracingMark`
5. `TraceSchedSwitch`
6. `Trace`

Если строка невалидна, возвращается `None`.

## Важное архитектурное решение

Typed-классы используют композицию, а не наследование:

- `TraceSchedSwitch` содержит `base: Trace`
- `TracingMark` содержит `base: Trace`
- `TraceMarkBegin` содержит `mark: TracingMark`
- `TraceReceiveVsync` содержит `begin: TraceMarkBegin`

Это сделано потому, что для `PyO3` такая модель проще и надежнее, чем настоящее Python inheritance.

Общие поля не должны дублироваться в каждом typed-классе.
Доступ к ним должен идти через вложенные объекты:

- `sched_switch.base.timestamp`
- `tracing_mark.base.payload_raw`
- `receive_vsync.begin.mark.base.thread_name`

## Чего не хватает сейчас

### 1. `tracing_mark` еще не доведен до шаблонной модели `{...}`

Это текущий технический долг.

Мы уже договорились, что для простых payload-форматов проект использует единый шаблон через `{field}` и `FieldSpec`.

Сейчас `TraceMarkBegin`, `TraceMarkEnd` и `TraceReceiveVsync` пока еще разбираются обычными regex, а не через `PayloadTemplate`.

Это надо исправить:

- `TraceMarkBegin` и `TraceMarkEnd` должны перейти на шаблонный parse
- сервисная часть вроде `[ExtraInfo]` должна моделироваться как игнорируемая группа
- для `tracing_mark` надо сохранить ту же идею, что и для остальных событий:
  - одно описание формата
  - parse из него
  - `to_string()` из него

### 2. Нужна явная поддержка ignored/service group кроме `{ws}`

Сейчас отдельная игнорируемая группа вроде `[ExtraInfo]` еще не оформлена как общий механизм в `PayloadTemplate`.

Нужно добавить что-то вроде:

- игнорируемой группы по regex
- optional игнорируемой группы

чтобы это использовалось не только в `tracing_mark`, но и в других будущих событиях.

### 3. Новые типы полей пока не используются в реальных trace-классах

В `FieldSpec` уже заложены:

- `u32`
- `i32`
- `f64`
- `choice(...)`

Но в текущих классах реально используется только часть этого набора.

## Ближайшие следующие шаги

1. Перевести `tracing_mark` на `PayloadTemplate`
2. Добавить общий механизм ignored/service group
3. Добавить новые типы событий:
   - `sched_wakeup`
   - `sched_waking`
   - дополнительные `tracing_mark`-подтипы
4. Подумать о registry для typed event-классов, чтобы фабричный parse не рос вручную

## Тесты

Тесты лежат рядом с кодом:

- `trace.rs`
- `sched_switch.rs`
- `tracing_mark.rs`
- `payload_template.rs`

Ожидаемый способ проверки:

```bash
cargo test -q
```
