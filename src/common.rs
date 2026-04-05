use lexical_core::parse;
use memchr::memmem;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use regex::Captures;

use crate::format_registry::FormatRegistry;

// Re-export from base_parser (memchr-based, no regex)
pub(crate) use crate::base_parser::BaseTraceParts;
pub(crate) use crate::base_parser::extract_event_name;

pub(crate) fn parse_base_parts(line: &str) -> Option<BaseTraceParts> {
    BaseTraceParts::parse(line)
}

pub(crate) trait EventType {
    const EVENT_NAME: &'static str;
    const EVENT_ALIASES: &'static [&'static str] = &[];

    fn matches_event_name(name: &str) -> bool {
        name == Self::EVENT_NAME || Self::EVENT_ALIASES.contains(&name)
    }
}

pub(crate) trait FastMatch: EventType {
    /// Маркеры для простой проверки payload (автоматическая SIMD проверка)
    const PAYLOAD_MARKERS: &'static [&'static [u8]] = &[];

    /// Быстрая проверка: event_name && маркеры && payload_quick_check
    /// Все три условия должны выполниться
    fn quick_check(line: &str) -> bool {
        let Some(event_name) = extract_event_name(line) else {
            return false;
        };

        // 1. Проверка event_name
        if !Self::matches_event_name(event_name) {
            return false;
        }

        // 2. Проверка payload маркеров (SIMD)
        if !Self::PAYLOAD_MARKERS
            .iter()
            .all(|m| memmem::find(line.as_bytes(), m).is_some())
        {
            return false;
        }

        // 3. Кастомная проверка payload (если переопределена)
        Self::payload_quick_check(line)
    }

    /// Кастомная проверка payload (для сложной логики)
    /// По умолчанию всегда true
    fn payload_quick_check(_line: &str) -> bool {
        true
    }
}

pub(crate) trait TemplateEvent: EventType {
    /// Словарь форматов для этого события
    fn formats() -> &'static FormatRegistry;

    /// Детекция формата по payload
    /// По умолчанию возвращает 0 (формат по умолчанию)
    fn detect_format(_payload: &str) -> u8 {
        0
    }

    /// Парсинг полей из captures с учётом формата
    fn parse_payload(parts: BaseTraceParts, captures: &Captures<'_>, format_id: u8) -> Option<Self>
    where
        Self: Sized;

    /// Рендер полей в payload с учётом формата
    fn render_payload(&self) -> PyResult<String>;
}

pub(crate) fn parse_template_event<T: TemplateEvent>(line: &str) -> Option<T> {
    let parts = parse_base_parts(line)?;
    let payload_raw = parts.payload_raw.clone();
    let format_id = T::detect_format(&payload_raw);
    let template = T::formats().template(format_id)?;
    let captures = template.captures(&payload_raw)?;
    T::parse_payload(parts, &captures, format_id)
}

pub(crate) fn cap_str(captures: &Captures<'_>, name: &str) -> Option<String> {
    Some(captures.name(name)?.as_str().to_owned())
}

pub(crate) fn cap_parse<T: lexical_core::FromLexical>(
    captures: &Captures<'_>,
    name: &str,
) -> Option<T> {
    parse(captures.name(name)?.as_str().as_bytes()).ok()
}

pub(crate) fn validate_timestamp(value: f64) -> PyResult<f64> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(PyValueError::new_err(
            "timestamp must be a finite non-negative number of seconds",
        ))
    }
}
