use once_cell::sync::Lazy;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use regex::Regex;

use crate::payload_template::PayloadTemplate;

pub(crate) static BASE_TRACE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<thread_name>.+)-(?P<tid>\d+)\s+\(\s*(?P<tgid>\d+)\)\s+\[(?P<cpu>\d+)\]\s+(?P<flags>\S+)\s+(?P<timestamp>\d+(?:\.\d+)?):\s+(?P<event_name>[^:]+):\s*(?P<payload>.*)$",
    )
    .expect("base trace regex must compile")
});

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
            tid: captures.name("tid")?.as_str().parse().ok()?,
            tgid: captures.name("tgid")?.as_str().parse().ok()?,
            cpu: captures.name("cpu")?.as_str().parse().ok()?,
            flags: captures.name("flags")?.as_str().to_owned(),
            timestamp: captures.name("timestamp")?.as_str().parse().ok()?,
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

pub(crate) trait TemplateEvent: EventType {
    fn template() -> &'static PayloadTemplate;
}

pub(crate) fn can_parse_event<T: EventType>(line: &str) -> bool {
    let Some(parts) = parse_base_parts(line) else {
        return false;
    };
    parts.event_name == T::EVENT_NAME
}

pub(crate) fn parse_event<T: EventType>(line: &str) -> Option<BaseTraceParts> {
    let parts = parse_base_parts(line)?;
    if parts.event_name != T::EVENT_NAME {
        return None;
    }
    Some(parts)
}

pub(crate) fn can_parse_template_event<T: TemplateEvent>(line: &str) -> bool {
    let Some(parts) = parse_base_parts(line) else {
        return false;
    };
    parts.event_name == T::EVENT_NAME && T::template().is_match(&parts.payload_raw)
}

pub(crate) fn parse_template_event<T: TemplateEvent>(line: &str) -> Option<(BaseTraceParts, String)> {
    let parts = parse_event::<T>(line)?;
    if !T::template().is_match(&parts.payload_raw) {
        return None;
    }
    let payload_raw = parts.payload_raw.clone();
    Some((parts, payload_raw))
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
