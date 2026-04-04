# Пример: Кастомный regex через `#[field(regex = "...")]`

Использование `#[field(regex = "...")]` для нестандартного парсинга поля.

```rust
use trace_parser_macros::TraceEvent;

#[trace_event(name = "custom_event")]
#[define_template("code={code} value={value}")]
#[derive(TraceEvent)]
struct TraceCustomCode {
    // Парсится как строка по кастомному regex
    #[field(regex = r"[A-Z]{2}\d{3}")]
    code: String,

    #[field]
    value: u32,
}
```

## Что генерируется

```rust
// В FieldSpec:
FieldSpec::custom(r"[A-Z]{2}\d{3}")

// В парсинге:
code: cap_str(captures, "code")?,  // regex: [A-Z]{2}\d{3}
```

Без `regex` использовался бы `FieldSpec::string()` → `.+?` (ленивое совпадение).

## Зачем

Когда стандартный `string()` (`.+?`) захватывает слишком много или слишком мало. Кастомный regex позволяет точно задать формат поля.

## Примеры regex

| Задача | regex |
|--------|-------|
| Код `AB123` | `r"[A-Z]{2}\d{3}"` |
| IP-адрес | `r"\d+\.\d+\.\d+\.\d+"` |
| UUID | `r"[0-9a-f-]{36}"` |
| Hex-значение | `r"0x[0-9a-fA-F]+"` |
| Целое с кастомным диапазоном | `r"\d{3}"` (ровно 3 цифры) |

## Использование

```rust
let line = "task-1 (1) [0] .... 1.0: custom_event: code=AB123 value=42";
let event = TraceCustomCode::parse(line).unwrap();
assert_eq!(event.code, "AB123");
assert_eq!(event.value, 42);

// Неправильный формат кода → None
let line2 = "task-1 (1) [0] .... 1.0: custom_event: code=ab123 value=42";
assert!(TraceCustomCode::parse(line2).is_none());
```
