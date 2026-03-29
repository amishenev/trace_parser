use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use regex::{Captures, Regex};
use std::sync::LazyLock;
use lexical_core::parse;
use memchr::memmem;

use crate::format_registry::FormatRegistry;

pub(crate) static BASE_TRACE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<thread_name>.+)-(?P<tid>\d+)\s+\(\s*(?P<tgid>\d+)\)\s+\[(?P<cpu>\d+)\]\s+(?P<flags>\S+)\s+(?P<timestamp>\d+(?:\.\d+)?):\s+(?P<event_name>[^:]+):\s*(?P<payload>.*)$",
    )
    .expect("base trace regex must compile")
});

#[derive(Clone)]
pub(crate) struct BaseTraceParts {
    pub(crate) thread_name: String,
    pub(crate) tid: u32,
    pub(crate) tgid: u32,
    pub(crate) cpu: u32,
    pub(crate) flags: String,
    pub(crate) timestamp: f64,
    pub(crate) event_name: String,
    pub(crate) payload_raw: String,
}

impl BaseTraceParts {
    pub(crate) fn parse(line: &str) -> Option<Self> {
        let captures = BASE_TRACE_RE.captures(line)?;
        Some(Self {
            thread_name: captures.name("thread_name")?.as_str().to_owned(),
            tid: parse(captures.name("tid")?.as_str().as_bytes()).ok()?,
            tgid: parse(captures.name("tgid")?.as_str().as_bytes()).ok()?,
            cpu: parse(captures.name("cpu")?.as_str().as_bytes()).ok()?,
            flags: captures.name("flags")?.as_str().to_owned(),
            timestamp: parse(captures.name("timestamp")?.as_str().as_bytes()).ok()?,
            event_name: captures.name("event_name")?.as_str().trim().to_owned(),
            payload_raw: captures.name("payload")?.as_str().to_owned(),
        })
    }
}

pub(crate) fn parse_base_parts(line: &str) -> Option<BaseTraceParts> {
    BaseTraceParts::parse(line)
}

pub(crate) trait EventType {
    const EVENT_NAME: &'static str;
}

pub(crate) trait FastMatch: EventType {
    fn quick_check(line: &str) -> bool {
        contains_event_name(line, Self::EVENT_NAME) && Self::payload_quick_check(line)
    }

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
    fn parse_payload(
        parts: BaseTraceParts,
        captures: &Captures<'_>,
        format_id: u8,
    ) -> Option<Self>
    where
        Self: Sized;

    /// Рендер полей в payload с учётом формата
    fn render_payload(&self) -> PyResult<String>;
}

pub(crate) fn contains_event_name(line: &str, event_name: &str) -> bool {
    let needle = event_name.as_bytes();
    let bytes = line.as_bytes();
    let mut start = 0;

    while start + needle.len() <= bytes.len() {
        let Some(offset) = memmem::find(&bytes[start..], needle) else {
            return false;
        };
        let index = start + offset;
        let before_ok = index >= 2 && &bytes[index - 2..index] == b": ";
        let after_index = index + needle.len();
        let after_ok = after_index + 2 <= bytes.len() && &bytes[after_index..after_index + 2] == b": ";
        if before_ok && after_ok {
            return true;
        }
        start = index + 1;
    }

    false
}

pub(crate) fn contains_all(line: &str, needles: &[&str]) -> bool {
    needles.iter().all(|needle| line.contains(needle))
}

pub(crate) fn contains_any(line: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| line.contains(needle))
}

pub(crate) fn parse_event<T: EventType>(line: &str) -> Option<BaseTraceParts> {
    let parts = parse_base_parts(line)?;
    if parts.event_name != T::EVENT_NAME {
        return None;
    }
    Some(parts)
}

pub(crate) fn parse_template_event<T: TemplateEvent>(line: &str) -> Option<T> {
    let parts = parse_event::<T>(line)?;
    let payload_raw = parts.payload_raw.clone();

    // Детекция формата
    let format_id = T::detect_format(&payload_raw);

    // Выбор шаблона по формату
    let template = T::formats().template(format_id)?;

    // Парсинг через regex
    let captures = template.captures(&payload_raw)?;

    // Создание объекта
    T::parse_payload(parts, &captures, format_id)
}

pub(crate) fn cap_str(captures: &Captures<'_>, name: &str) -> Option<String> {
    Some(captures.name(name)?.as_str().to_owned())
}

pub(crate) fn cap_parse<T: lexical_core::FromLexical>(captures: &Captures<'_>, name: &str) -> Option<T> {
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
