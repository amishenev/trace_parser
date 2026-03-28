use once_cell::sync::Lazy;
use pyo3::prelude::*;
use regex::Captures;
use std::collections::HashMap;

use crate::common::{
    cap_parse, cap_str, parse_template_event, validate_timestamp, BaseTraceParts, EventType,
    FastMatch, TemplateEvent,
};
use crate::format_registry::{FormatRegistry, FormatSpec};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::trace::Trace;

pub(crate) static BEGIN_TEMPLATE: Lazy<PayloadTemplate> = Lazy::new(|| {
    PayloadTemplate::new(
        "B|{trace_mark_tgid}|{payload}",
        &[
            ("trace_mark_tgid", FieldSpec::u32()),
            ("payload", FieldSpec::custom(r".*")),
        ],
    )
});

pub(crate) static BEGIN_FORMATS: Lazy<FormatRegistry> = Lazy::new(|| {
    FormatRegistry::new(vec![
        FormatSpec {
            kind: 0,
            template: &BEGIN_TEMPLATE,
        },
    ])
});

pub(crate) static END_TEMPLATE: Lazy<PayloadTemplate> = Lazy::new(|| {
    PayloadTemplate::new(
        "E|{trace_mark_tgid}|{payload}",
        &[
            ("trace_mark_tgid", FieldSpec::u32()),
            ("payload", FieldSpec::custom(r".*")),
        ],
    )
});

pub(crate) static END_FORMATS: Lazy<FormatRegistry> = Lazy::new(|| {
    FormatRegistry::new(vec![
        FormatSpec {
            kind: 0,
            template: &END_TEMPLATE,
        },
    ])
});

use crate::common::parse_event;

pub(crate) fn contains_begin_marker(line: &str) -> bool {
    line.contains(" B|") || line.contains(": B|") || line.contains("tracing_mark_write: B|")
}

pub(crate) fn contains_end_marker(line: &str) -> bool {
    line.contains(" E|") || line.contains(": E|") || line.contains("tracing_mark_write: E|")
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct TracingMark {
    #[pyo3(get)]
    pub(crate) base: Trace,
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct TraceMarkBegin {
    #[pyo3(get)]
    pub(crate) base: Trace,
    #[pyo3(get, set)]
    pub(crate) format_id: u8,
    #[pyo3(get, set)]
    pub(crate) trace_mark_tgid: u32,
    #[pyo3(get, set)]
    pub(crate) payload: String,
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct TraceMarkEnd {
    #[pyo3(get)]
    pub(crate) base: Trace,
    #[pyo3(get, set)]
    pub(crate) format_id: u8,
    #[pyo3(get, set)]
    pub(crate) trace_mark_tgid: u32,
    #[pyo3(get, set)]
    pub(crate) payload: String,
}

impl EventType for TracingMark {
    const EVENT_NAME: &'static str = "tracing_mark_write";
}

impl FastMatch for TracingMark {}

impl EventType for TraceMarkBegin {
    const EVENT_NAME: &'static str = "tracing_mark_write";
}

impl FastMatch for TraceMarkBegin {
    fn payload_quick_check(line: &str) -> bool {
        contains_begin_marker(line)
    }
}

impl TemplateEvent for TraceMarkBegin {
    fn formats() -> &'static FormatRegistry {
        &BEGIN_FORMATS
    }

    fn parse_payload(
        parts: BaseTraceParts,
        captures: &Captures<'_>,
        _format_id: u8,
    ) -> Option<Self> {
        Some(Self {
            base: Trace::from_parts(parts),
            format_id: 0,
            trace_mark_tgid: cap_parse(captures, "trace_mark_tgid")?,
            payload: cap_str(captures, "payload")?,
        })
    }

    fn render_payload(&self) -> PyResult<String> {
        let template = Self::formats().template(0).unwrap();
        let values = HashMap::from([
            ("trace_mark_tgid", TemplateValue::U32(self.trace_mark_tgid)),
            ("payload", TemplateValue::Str(&self.payload)),
        ]);
        Ok(template
            .format(&values)
            .expect("trace mark begin template must render"))
    }
}

impl EventType for TraceMarkEnd {
    const EVENT_NAME: &'static str = "tracing_mark_write";
}

impl FastMatch for TraceMarkEnd {
    fn payload_quick_check(line: &str) -> bool {
        contains_end_marker(line)
    }
}

impl TemplateEvent for TraceMarkEnd {
    fn formats() -> &'static FormatRegistry {
        &END_FORMATS
    }

    fn parse_payload(
        parts: BaseTraceParts,
        captures: &Captures<'_>,
        _format_id: u8,
    ) -> Option<Self> {
        Some(Self {
            base: Trace::from_parts(parts),
            format_id: 0,
            trace_mark_tgid: cap_parse(captures, "trace_mark_tgid")?,
            payload: cap_str(captures, "payload")?,
        })
    }

    fn render_payload(&self) -> PyResult<String> {
        let template = Self::formats().template(0).unwrap();
        let values = HashMap::from([
            ("trace_mark_tgid", TemplateValue::U32(self.trace_mark_tgid)),
            ("payload", TemplateValue::Str(&self.payload)),
        ]);
        Ok(template
            .format(&values)
            .expect("trace mark end template must render"))
    }
}

#[pymethods]
impl TracingMark {
    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        Self::quick_check(line)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        if !Self::quick_check(line) {
            return None;
        }
        let parts = parse_event::<Self>(line)?;
        Some(Self {
            base: Trace::from_parts(parts),
        })
    }

    pub(crate) fn payload_to_string(&self) -> PyResult<String> {
        self.base.payload_to_string()
    }

    pub(crate) fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.base.timestamp)?;
        Ok(self.base.to_string_with_payload(&self.payload_to_string()?))
    }
}

#[pymethods]
impl TraceMarkBegin {
    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        Self::quick_check(line)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        if !Self::can_be_parsed(line) {
            return None;
        }
        parse_template_event::<Self>(line)
    }

    pub(crate) fn payload_to_string(&self) -> PyResult<String> {
        self.render_payload()
    }

    pub(crate) fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.base.timestamp)?;
        Ok(self.base.to_string_with_payload(&self.payload_to_string()?))
    }
}

#[pymethods]
impl TraceMarkEnd {
    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        Self::quick_check(line)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        if !Self::can_be_parsed(line) {
            return None;
        }
        parse_template_event::<Self>(line)
    }

    pub(crate) fn payload_to_string(&self) -> PyResult<String> {
        self.render_payload()
    }

    pub(crate) fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.base.timestamp)?;
        Ok(self.base.to_string_with_payload(&self.payload_to_string()?))
    }
}

#[cfg(test)]
mod tests {
    use crate::{TraceMarkBegin, TraceMarkEnd, TracingMark};

    #[test]
    fn tracing_mark_accepts_any_payload() {
        let line = "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: anything at all";
        let mark = TracingMark::parse(line).expect("tracing mark must parse");
        assert_eq!(mark.base.event_name, "tracing_mark_write");
        assert_eq!(mark.base.payload_raw, "anything at all");
        assert_eq!(
            mark.payload_to_string().expect("payload_to_string must work"),
            "anything at all"
        );
    }

    #[test]
    fn trace_mark_begin_parses_generic_begin_payload() {
        let line =
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|some_custom_message";
        let mark = TraceMarkBegin::parse(line).expect("begin mark must parse");
        assert_eq!(mark.trace_mark_tgid, 10);
        assert_eq!(mark.payload, "some_custom_message");
        assert_eq!(
            mark.payload_to_string().expect("payload_to_string must work"),
            "B|10|some_custom_message"
        );
        assert_eq!(
            mark.to_string().expect("to_string must work"),
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|some_custom_message"
        );
    }

    #[test]
    fn trace_mark_end_parses_generic_end_payload() {
        let line = "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: E|10|done";
        let mark = TraceMarkEnd::parse(line).expect("end mark must parse");
        assert_eq!(mark.trace_mark_tgid, 10);
        assert_eq!(mark.payload, "done");
    }
}
