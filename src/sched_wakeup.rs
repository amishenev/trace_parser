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
        "comm={comm} pid={pid} prio={prio} target_cpu={target_cpu}",
        &[
            ("comm", FieldSpec::string()),
            ("pid", FieldSpec::u32()),
            ("prio", FieldSpec::i32()),
            ("target_cpu", FieldSpec::custom(r"\d{3}")),
        ],
    )
});

#[pyclass]
#[derive(Clone, Debug)]
pub struct TraceSchedWakeup {
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
    pub(crate) target_cpu: u32,
}

impl EventType for TraceSchedWakeup {
    const EVENT_NAME: &'static str = "sched_wakeup";
}

impl FastMatch for TraceSchedWakeup {}

impl TemplateEvent for TraceSchedWakeup {
    fn template() -> &'static PayloadTemplate {
        &TEMPLATE
    }
}

impl EventType for TraceSchedWakeupNew {
    const EVENT_NAME: &'static str = "sched_wakeup_new";
}

impl FastMatch for TraceSchedWakeupNew {}

impl TemplateEvent for TraceSchedWakeupNew {
    fn template() -> &'static PayloadTemplate {
        &TEMPLATE
    }
}

#[pymethods]
impl TraceSchedWakeup {
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
                target_cpu: cap_parse(captures, "target_cpu")?,
            })
        })
    }

    pub(crate) fn payload_to_string(&self) -> PyResult<String> {
        let target_cpu = format!("{:03}", self.target_cpu);
        let values = HashMap::from([
            ("comm", TemplateValue::Str(&self.comm)),
            ("pid", TemplateValue::U32(self.pid)),
            ("prio", TemplateValue::I32(self.prio)),
            ("target_cpu", TemplateValue::Str(&target_cpu)),
        ]);
        Ok(Self::template()
            .format(&values)
            .expect("sched_wakeup template must render"))
    }

    pub(crate) fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.base.timestamp)?;
        Ok(self.base.to_string_with_payload(&self.payload_to_string()?))
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct TraceSchedWakeupNew {
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
    pub(crate) target_cpu: u32,
}

#[pymethods]
impl TraceSchedWakeupNew {
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
                target_cpu: cap_parse(captures, "target_cpu")?,
            })
        })
    }

    pub(crate) fn payload_to_string(&self) -> PyResult<String> {
        let target_cpu = format!("{:03}", self.target_cpu);
        let values = HashMap::from([
            ("comm", TemplateValue::Str(&self.comm)),
            ("pid", TemplateValue::U32(self.pid)),
            ("prio", TemplateValue::I32(self.prio)),
            ("target_cpu", TemplateValue::Str(&target_cpu)),
        ]);
        Ok(Self::template()
            .format(&values)
            .expect("sched_wakeup_new template must render"))
    }

    pub(crate) fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.base.timestamp)?;
        Ok(self.base.to_string_with_payload(&self.payload_to_string()?))
    }
}

#[cfg(test)]
mod tests {
    use crate::{TraceSchedWakeup, TraceSchedWakeupNew};

    #[test]
    fn sched_wakeup_parses() {
        let line = "kworker-123 ( 123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000";
        let trace = TraceSchedWakeup::parse(line).expect("sched_wakeup must parse");
        assert_eq!(trace.comm, "bash");
        assert_eq!(trace.pid, 1977);
        assert_eq!(trace.prio, 120);
        assert_eq!(trace.target_cpu, 0);
        assert_eq!(
            trace.payload_to_string().expect("payload_to_string must work"),
            "comm=bash pid=1977 prio=120 target_cpu=000"
        );
        assert_eq!(
            trace.to_string().expect("to_string must work"),
            "kworker-123 (123) [000] .... 12345.679001: sched_wakeup: comm=bash pid=1977 prio=120 target_cpu=000"
        );
    }

    #[test]
    fn sched_wakeup_new_parses() {
        let line = "kworker-123 ( 123) [000] .... 12345.679001: sched_wakeup_new: comm=bash pid=1977 prio=120 target_cpu=000";
        let trace = TraceSchedWakeupNew::parse(line).expect("sched_wakeup_new must parse");
        assert_eq!(trace.comm, "bash");
        assert_eq!(trace.pid, 1977);
        assert_eq!(trace.prio, 120);
        assert_eq!(trace.target_cpu, 0);
    }
}
