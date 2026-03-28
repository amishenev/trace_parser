use once_cell::sync::Lazy;
use pyo3::prelude::*;
use std::collections::HashMap;

use crate::common::{parse_base_parts, validate_timestamp};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::trace::Trace;

pub(crate) static TRACE_MARK_BEGIN_TEMPLATE: Lazy<PayloadTemplate> = Lazy::new(|| {
    PayloadTemplate::new(
        "B|{trace_mark_tgid}|{payload}",
        &[
            ("trace_mark_tgid", FieldSpec::u32()),
            ("payload", FieldSpec::custom(r".*")),
        ],
    )
});

pub(crate) static TRACE_MARK_END_TEMPLATE: Lazy<PayloadTemplate> = Lazy::new(|| {
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
        let Some(parts) = parse_base_parts(line) else {
            return false;
        };
        parts.event_name == "tracing_mark_write"
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        let parts = parse_base_parts(line)?;
        if parts.event_name != "tracing_mark_write" {
            return None;
        }
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
        let Some(mark) = TracingMark::parse(line) else {
            return false;
        };
        TRACE_MARK_BEGIN_TEMPLATE.is_match(&mark.base.payload_raw)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        let mark = TracingMark::parse(line)?;
        let captures = TRACE_MARK_BEGIN_TEMPLATE.captures(&mark.base.payload_raw)?;
        let trace_mark_tgid = captures.name("trace_mark_tgid")?.as_str().parse().ok()?;
        let payload = captures.name("payload")?.as_str().to_owned();
        Some(Self {
            mark,
            trace_mark_tgid,
            payload,
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

        Ok(TRACE_MARK_BEGIN_TEMPLATE
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
        let Some(mark) = TracingMark::parse(line) else {
            return false;
        };
        TRACE_MARK_END_TEMPLATE.is_match(&mark.base.payload_raw)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        let mark = TracingMark::parse(line)?;
        let captures = TRACE_MARK_END_TEMPLATE.captures(&mark.base.payload_raw)?;
        let trace_mark_tgid = captures.name("trace_mark_tgid")?.as_str().parse().ok()?;
        let payload = captures.name("payload")?.as_str().to_owned();
        Some(Self {
            mark,
            trace_mark_tgid,
            payload,
        })
    }

    pub(crate) fn payload_to_string(&self) -> PyResult<String> {
        let values = HashMap::from([
            ("trace_mark_tgid", TemplateValue::U32(self.trace_mark_tgid)),
            ("payload", TemplateValue::Str(&self.payload)),
        ]);

        Ok(TRACE_MARK_END_TEMPLATE
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
