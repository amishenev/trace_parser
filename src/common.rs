use lexical_core::parse;
use memchr::memmem;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use regex::{Captures, Regex};
use std::sync::LazyLock;

use crate::format_registry::FormatRegistry;

pub(crate) static BASE_TRACE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<thread_name>.+)-(?P<tid>\d+)\s+\(\s*(?P<tgid>\d+|-+)\)\s+\[(?P<cpu>\d+)\]\s+(?P<flags>\S+)\s+(?P<timestamp>\d+(?:\.\d+)?):\s+(?P<event_name>[^:]+):\s*(?P<payload>.*)$",
    )
    .expect("base trace regex must compile")
});

#[derive(Clone)]
pub struct BaseTraceParts {
    pub thread_name: String,
    pub thread_tid: u32,
    pub thread_tgid: Option<u32>,
    pub cpu: u32,
    pub flags: String,
    pub timestamp: f64,
    pub event_name: String,
    pub payload_raw: String,
}

impl BaseTraceParts {
    pub fn parse(line: &str) -> Option<Self> {
        let captures = BASE_TRACE_RE.captures(line)?;
        Some(Self {
            thread_name: captures.name("thread_name")?.as_str().to_owned(),
            thread_tid: parse(captures.name("tid")?.as_str().as_bytes()).ok()?,
            thread_tgid: parse_thread_tgid_token(captures.name("tgid")?.as_str())?,
            cpu: parse(captures.name("cpu")?.as_str().as_bytes()).ok()?,
            flags: captures.name("flags")?.as_str().to_owned(),
            timestamp: parse(captures.name("timestamp")?.as_str().as_bytes()).ok()?,
            event_name: captures.name("event_name")?.as_str().trim().to_owned(),
            payload_raw: captures.name("payload")?.as_str().to_owned(),
        })
    }
}

fn parse_thread_tgid_token(raw: &str) -> Option<Option<u32>> {
    if raw.bytes().all(|b| b == b'-') {
        return Some(None);
    }
    parse(raw.as_bytes()).ok().map(Some)
}

pub fn parse_base_parts(line: &str) -> Option<BaseTraceParts> {
    BaseTraceParts::parse(line)
}

/// Извлечь event_name из строки трассировки
pub fn extract_event_name(line: &str) -> Option<&str> {
    // Используем SIMD поиск через memchr
    let colon_pos = memmem::find(line.as_bytes(), b": ")? + 2;
    let rest = &line[colon_pos..];
    let end_pos = memmem::find(rest.as_bytes(), b": ")?;
    Some(rest[..end_pos].trim())
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
