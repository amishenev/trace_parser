# Пример: Enum через `#[derive(TraceEnum)]`

Использование `#[derive(TraceEnum)]` для генерации `Display`, `FromStr` и `TraceEnum`.

```rust
use trace_parser_macros::TraceEnum;

#[derive(Debug, Clone, PartialEq, TraceEnum)]
enum PrevState {
    #[value("R")]
    Running,

    #[value("S")]
    Sleeping,

    #[value("D")]
    Uninterruptible,

    #[value("T")]
    Stopped,

    #[value("Z")]
    Zombie,
}
```

## Что генерируется

```rust
// Display
impl std::fmt::Display for PrevState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrevState::Running => write!(f, "R"),
            PrevState::Sleeping => write!(f, "S"),
            // ...
        }
    }
}

// FromStr
impl std::str::FromStr for PrevState {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "R" => Ok(PrevState::Running),
            "S" => Ok(PrevState::Sleeping),
            // ...
            _ => Err(format!("Unknown PrevState value: {}", s)),
        }
    }
}

// TraceEnum (для использования в FieldSpec::choice)
impl ::trace_parser::payload_template::TraceEnum for PrevState {
    fn variants() -> &'static [&'static str] { &["R", "S", "D", "T", "Z"] }
}
```

## Зачем

- **Типобезопасность** — вместо `String` используется enum
- **Display/FromStr** — конвертация в/из строки трассировки
- **TraceEnum** — интеграция с `FieldSpec::choice` (варианты enum → regex alternatives)

## Использование

```rust
// Из строки
let state: PrevState = "S".parse().unwrap();
assert_eq!(state, PrevState::Sleeping);

// В строку
assert_eq!(PrevState::Running.to_string(), "R");

// Ошибка парсинга
assert!("X".parse::<PrevState>().is_err());
```
