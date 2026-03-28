use std::collections::HashMap;

use once_cell::sync::Lazy;
use pyo3::prelude::*;

use crate::common::{parse_base_parts, validate_timestamp};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::trace::Trace;

static SCHED_PROCESS_EXIT_TEMPLATE: Lazy<PayloadTemplate> = Lazy::new(|| {
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

#[pymethods]
impl TraceSchedProcessExit {
    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        let Some(parts) = parse_base_parts(line) else {
            return false;
        };
        parts.event_name == "sched_process_exit"
            && SCHED_PROCESS_EXIT_TEMPLATE.is_match(&parts.payload_raw)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        let parts = parse_base_parts(line)?;
        if parts.event_name != "sched_process_exit" {
            return None;
        }
        let captures = SCHED_PROCESS_EXIT_TEMPLATE.captures(&parts.payload_raw)?;
        let comm = captures.name("comm")?.as_str().to_owned();
        let pid = captures.name("pid")?.as_str().parse().ok()?;
        let prio = captures.name("prio")?.as_str().parse().ok()?;
        let group_dead = matches!(captures.name("group_dead")?.as_str(), "1");
        Some(Self {
            base: Trace::from_parts(parts),
            format_id: "default".to_owned(),
            comm,
            pid,
            prio,
            group_dead,
        })
    }

    pub(crate) fn payload_to_string(&self) -> PyResult<String> {
        let values = HashMap::from([
            ("comm", TemplateValue::Str(&self.comm)),
            ("pid", TemplateValue::U32(self.pid)),
            ("prio", TemplateValue::I32(self.prio)),
            ("group_dead", TemplateValue::BoolInt(self.group_dead)),
        ]);
        Ok(SCHED_PROCESS_EXIT_TEMPLATE
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
