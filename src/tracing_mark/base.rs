use once_cell::sync::Lazy;
use pyo3::prelude::*;
use std::collections::HashMap;

use crate::common::{
    can_parse_event, can_parse_template_event, cap_parse, cap_str, parse_event,
    parse_template_event, validate_timestamp, EventType, TemplateEvent,
};
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

impl EventType for TracingMark {
    const EVENT_NAME: &'static str = "tracing_mark_write";
}

impl EventType for TraceMarkBegin {
    const EVENT_NAME: &'static str = "tracing_mark_write";
}

impl TemplateEvent for TraceMarkBegin {
    fn template() -> &'static PayloadTemplate {
        &BEGIN_TEMPLATE
    }
}

impl EventType for TraceMarkEnd {
    const EVENT_NAME: &'static str = "tracing_mark_write";
}

impl TemplateEvent for TraceMarkEnd {
    fn template() -> &'static PayloadTemplate {
        &END_TEMPLATE
    }
}

pub(crate) static END_TEMPLATE: Lazy<PayloadTemplate> = Lazy::new(|| {
    PayloadTemplate::new(
        "E|{trace_mark_tgid}|{payload}",
        &[
            ("trace_mark_tgid", FieldSpec::u32()),
            ("payload", FieldSpec::custom(r".*")),
        ],
    )
});

#[pyclass]
#[derive(Clone, Debug)]
pub struct TracingMark {
    #[pyo3(get)]
    pub(crate) base: Trace,
}

#[pymethods]
impl TracingMark {
    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        can_parse_event::<Self>(line)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
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

#[pyclass]
#[derive(Clone, Debug)]
pub struct TraceMarkBegin {
    #[pyo3(get)]
    pub(crate) mark: TracingMark,
    #[pyo3(get, set)]
    pub(crate) trace_mark_tgid: u32,
    #[pyo3(get, set)]
    pub(crate) payload: String,
}

#[pymethods]
impl TraceMarkBegin {
    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        can_parse_template_event::<Self>(line)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        parse_template_event::<Self, _>(line, |parts, captures| {
            Some(Self {
                mark: TracingMark {
                    base: Trace::from_parts(parts),
                },
                trace_mark_tgid: cap_parse(captures, "trace_mark_tgid")?,
                payload: cap_str(captures, "payload")?,
            })
        })
    }

    pub(crate) fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.mark.base.timestamp)?;
        Ok(self.mark.base.to_string_with_payload(&self.payload_to_string()?))
    }

    pub(crate) fn payload_to_string(&self) -> PyResult<String> {
        let values = HashMap::from([
            ("trace_mark_tgid", TemplateValue::U32(self.trace_mark_tgid)),
            ("payload", TemplateValue::Str(&self.payload)),
        ]);

        Ok(Self::template()
            .format(&values)
            .expect("trace mark begin template must render"))
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct TraceMarkEnd {
    #[pyo3(get)]
    pub(crate) mark: TracingMark,
    #[pyo3(get, set)]
    pub(crate) trace_mark_tgid: u32,
    #[pyo3(get, set)]
    pub(crate) payload: String,
}

#[pymethods]
impl TraceMarkEnd {
    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        can_parse_template_event::<Self>(line)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        parse_template_event::<Self, _>(line, |parts, captures| {
            Some(Self {
                mark: TracingMark {
                    base: Trace::from_parts(parts),
                },
                trace_mark_tgid: cap_parse(captures, "trace_mark_tgid")?,
                payload: cap_str(captures, "payload")?,
            })
        })
    }

    pub(crate) fn payload_to_string(&self) -> PyResult<String> {
        let values = HashMap::from([
            ("trace_mark_tgid", TemplateValue::U32(self.trace_mark_tgid)),
            ("payload", TemplateValue::Str(&self.payload)),
        ]);

        Ok(Self::template()
            .format(&values)
            .expect("trace mark end template must render"))
    }

    pub(crate) fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.mark.base.timestamp)?;
        Ok(self.mark.base.to_string_with_payload(&self.payload_to_string()?))
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
        let line =
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: E|10|done";
        let mark = TraceMarkEnd::parse(line).expect("end mark must parse");
        assert_eq!(mark.trace_mark_tgid, 10);
        assert_eq!(mark.payload, "done");
    }
}
