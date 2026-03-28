use std::collections::HashMap;

use once_cell::sync::Lazy;
use pyo3::prelude::*;

use crate::common::{
    cap_parse, cap_str, parse_template_event, validate_timestamp, EventType, FastMatch,
    TemplateEvent,
};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::trace::Trace;

static TEMPLATE: Lazy<PayloadTemplate> = Lazy::new(|| {
    PayloadTemplate::new(
        "comm={comm} pid={pid} prio={prio} group_dead={group_dead}",
        &[
            ("comm", FieldSpec::string()),
            ("pid", FieldSpec::u32()),
            ("prio", FieldSpec::i32()),
            ("group_dead", FieldSpec::bool_int()),
        ],
    )
});

#[pyclass]
#[derive(Clone, Debug)]
pub struct TraceSchedProcessExit {
    #[pyo3(get)]
    pub(crate) base: Trace,
    #[pyo3(get, set)]
    pub(crate) format_id: String,
    #[pyo3(get, set)]
    pub(crate) comm: String,
    #[pyo3(get, set)]
    pub(crate) pid: u32,
    #[pyo3(get, set)]
    pub(crate) prio: i32,
    #[pyo3(get, set)]
    pub(crate) group_dead: bool,
}

impl EventType for TraceSchedProcessExit {
    const EVENT_NAME: &'static str = "sched_process_exit";
}

impl FastMatch for TraceSchedProcessExit {}

impl TemplateEvent for TraceSchedProcessExit {
    fn template() -> &'static PayloadTemplate {
        &TEMPLATE
    }
}

#[pymethods]
impl TraceSchedProcessExit {
    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        Self::quick_check(line)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        if !Self::can_be_parsed(line) {
            return None;
        }
        parse_template_event::<Self, _>(line, |parts, captures| {
            Some(Self {
                base: Trace::from_parts(parts),
                format_id: "default".to_owned(),
                comm: cap_str(captures, "comm")?,
                pid: cap_parse(captures, "pid")?,
                prio: cap_parse(captures, "prio")?,
                group_dead: cap_parse::<u8>(captures, "group_dead")? == 1,
            })
        })
    }

    pub(crate) fn payload_to_string(&self) -> PyResult<String> {
        let values = HashMap::from([
            ("comm", TemplateValue::Str(&self.comm)),
            ("pid", TemplateValue::U32(self.pid)),
            ("prio", TemplateValue::I32(self.prio)),
            ("group_dead", TemplateValue::BoolInt(self.group_dead)),
        ]);
        Ok(Self::template()
            .format(&values)
            .expect("sched_process_exit template must render"))
    }

    pub(crate) fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.base.timestamp)?;
        Ok(self.base.to_string_with_payload(&self.payload_to_string()?))
    }
}

#[cfg(test)]
mod tests {
    use crate::TraceSchedProcessExit;

    #[test]
    fn sched_process_exit_parses() {
        let line = "bash-1977 (12) [000] .... 12345.678901: sched_process_exit: comm=bash pid=1977 prio=120 group_dead=1";
        let trace = TraceSchedProcessExit::parse(line).expect("sched_process_exit must parse");
        assert_eq!(trace.comm, "bash");
        assert_eq!(trace.pid, 1977);
        assert_eq!(trace.prio, 120);
        assert!(trace.group_dead);
        assert_eq!(
            trace.payload_to_string().expect("payload_to_string must work"),
            "comm=bash pid=1977 prio=120 group_dead=1"
        );
        assert_eq!(
            trace.to_string().expect("to_string must work"),
            "bash-1977 (12) [000] .... 12345.678901: sched_process_exit: comm=bash pid=1977 prio=120 group_dead=1"
        );
    }
}
