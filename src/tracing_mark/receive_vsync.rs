use once_cell::sync::Lazy;
use pyo3::prelude::*;
use std::collections::HashMap;

use crate::common::validate_timestamp;
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use super::base::{TraceMarkBegin, TRACE_MARK_BEGIN_TEMPLATE};

static RECEIVE_VSYNC_TEMPLATE: Lazy<PayloadTemplate> = Lazy::new(|| {
    PayloadTemplate::new(
        "{?ignore:extra_info}ReceiveVsync {frame_number}",
        &[
            ("extra_info", FieldSpec::custom(r"\[[^]]+\]")),
            ("frame_number", FieldSpec::u32()),
        ],
    )
});

#[pyclass]
#[derive(Clone, Debug)]
pub struct TraceReceiveVsync {
    #[pyo3(get)]
    pub(crate) begin: TraceMarkBegin,
    #[pyo3(get, set)]
    pub(crate) frame_number: u32,
}

#[pymethods]
impl TraceReceiveVsync {
    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        let Some(begin) = TraceMarkBegin::parse(line) else {
            return false;
        };
        RECEIVE_VSYNC_TEMPLATE.is_match(&begin.payload)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        let begin = TraceMarkBegin::parse(line)?;
        let captures = RECEIVE_VSYNC_TEMPLATE.captures(&begin.payload)?;
        let frame_number = captures.name("frame_number")?.as_str().parse().ok()?;
        Some(Self {
            begin,
            frame_number,
        })
    }

    pub(crate) fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.begin.mark.base.timestamp)?;
        let payload_values = HashMap::from([("frame_number", TemplateValue::U32(self.frame_number))]);
        let payload = RECEIVE_VSYNC_TEMPLATE
            .format(&payload_values)
            .expect("receive vsync template must render");
        let begin_values = HashMap::from([
            ("trace_mark_tgid", TemplateValue::U32(self.begin.trace_mark_tgid)),
            ("payload", TemplateValue::Str(&payload)),
        ]);

        Ok(self.begin.mark.base.to_string_with_payload(
            &TRACE_MARK_BEGIN_TEMPLATE
                .format(&begin_values)
                .expect("trace mark begin template must render"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::TraceReceiveVsync;

    #[test]
    fn receive_vsync_parses_specific_begin_payload() {
        let line =
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|ReceiveVsync 42";
        let mark = TraceReceiveVsync::parse(line)
            .expect("receive vsync begin mark must parse");
        assert_eq!(mark.begin.trace_mark_tgid, 10);
        assert_eq!(mark.frame_number, 42);
        assert_eq!(
            mark.to_string().expect("to_string must work"),
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|ReceiveVsync 42"
        );
    }

    #[test]
    fn receive_vsync_ignores_service_prefix() {
        let line =
            "any_thread-232 (10) [010] .... 12345.678900: tracing_mark_write: B|10|[ExtraInfo]ReceiveVsync 42";
        let mark = TraceReceiveVsync::parse(line)
            .expect("receive vsync begin mark must parse");
        assert_eq!(mark.frame_number, 42);
    }
}
