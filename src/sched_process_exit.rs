use pyo3::prelude::*;
use regex::Captures;
use std::sync::LazyLock;

use crate::common::{
    cap_parse, cap_str, parse_template_event, validate_timestamp, BaseTraceParts, EventType,
    FastMatch, TemplateEvent,
};
use crate::format_registry::{FormatRegistry, FormatSpec};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::trace::Trace;

static TEMPLATE: LazyLock<PayloadTemplate> = LazyLock::new(|| {
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

static FORMATS: LazyLock<FormatRegistry> = LazyLock::new(|| {
    FormatRegistry::new(vec![
        FormatSpec {
            kind: 0,
            template: &TEMPLATE,
        },
    ])
});

#[pyclass]
#[derive(Clone, Debug, PartialEq)]
pub struct TraceSchedProcessExit {
    #[pyo3(get)]
    pub base: Trace,
    #[pyo3(get, set)]
    pub format_id: u8,
    #[pyo3(get, set)]
    pub comm: String,
    #[pyo3(get, set)]
    pub pid: u32,
    #[pyo3(get, set)]
    pub prio: i32,
    #[pyo3(get, set)]
    pub group_dead: bool,
}

impl EventType for TraceSchedProcessExit {
    const EVENT_NAME: &'static str = "sched_process_exit";
}

impl FastMatch for TraceSchedProcessExit {}

impl TemplateEvent for TraceSchedProcessExit {
    fn formats() -> &'static FormatRegistry {
        &FORMATS
    }

    fn parse_payload(
        parts: BaseTraceParts,
        captures: &Captures<'_>,
        _format_id: u8,
    ) -> Option<Self> {
        Some(Self {
            base: Trace::from_parts(parts),
            format_id: 0,
            comm: cap_str(captures, "comm")?,
            pid: cap_parse(captures, "pid")?,
            prio: cap_parse(captures, "prio")?,
            group_dead: cap_parse::<u8>(captures, "group_dead")? == 1,
        })
    }

    fn render_payload(&self) -> PyResult<String> {
        let template = Self::formats().template(0).unwrap();
        let values: [(&str, Option<TemplateValue>); 4] = [
            ("comm", Some(TemplateValue::Str(&self.comm))),
            ("pid", Some(TemplateValue::U32(self.pid))),
            ("prio", Some(TemplateValue::I32(self.prio))),
            ("group_dead", Some(TemplateValue::BoolInt(self.group_dead))),
        ];
        Ok(template
            .format(&values)
            .expect("sched_process_exit template must render"))
    }
}

#[pymethods]
impl TraceSchedProcessExit {
    #[new]
    #[pyo3(signature = (thread_name, tid, tgid, cpu, flags, timestamp, event_name, payload_raw, comm, pid, prio, group_dead, format_id))]
    pub fn new(
        thread_name: String,
        tid: u32,
        tgid: u32,
        cpu: u32,
        flags: String,
        timestamp: f64,
        event_name: String,
        payload_raw: String,
        comm: String,
        pid: u32,
        prio: i32,
        group_dead: bool,
        format_id: u8,
    ) -> PyResult<Self> {
        Ok(Self {
            base: Trace::new(
                thread_name,
                tid,
                tgid,
                cpu,
                flags,
                timestamp,
                event_name,
                payload_raw,
            )?,
            format_id,
            comm,
            pid,
            prio,
            group_dead,
        })
    }

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

    pub fn payload_to_string(&self) -> PyResult<String> {
        self.render_payload()
    }

    pub fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.base.timestamp)?;
        Ok(self.base.to_string_with_payload(&self.payload_to_string()?))
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "TraceSchedProcessExit(comm={:?}, pid={}, prio={}, group_dead={}, timestamp={})",
            self.comm, self.pid, self.prio, self.group_dead, self.base.timestamp
        ))
    }

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }

    fn __str__(&self) -> PyResult<String> {
        self.to_string()
    }

    fn __copy__(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<Self>> {
        Ok(slf.clone().into_pyobject(py)?.unbind())
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> PyResult<Py<Self>> {
        Ok(self.clone().into_pyobject(unsafe { Python::assume_attached() })?.unbind())
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
