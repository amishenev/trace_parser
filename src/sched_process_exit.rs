use pyo3::prelude::*;
use regex::Captures;
use std::sync::LazyLock;

use crate::common::{
    cap_parse, cap_str, parse_template_event, validate_timestamp, BaseTraceParts, EventType,
    FastMatch, TemplateEvent,
};
use crate::format_registry::{FormatRegistry, FormatSpec};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::trace::{extract_base_fields, format_trace_header};

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

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct TraceSchedProcessExit {
    #[pyo3(get, set)]
    pub thread_name: String,
    #[pyo3(get, set)]
    pub thread_tid: u32,
    #[pyo3(get, set)]
    pub thread_tgid: u32,
    #[pyo3(get, set)]
    pub cpu: u32,
    #[pyo3(get, set)]
    pub flags: String,
    #[pyo3(get, set)]
    pub timestamp: f64,
    #[pyo3(get)]
    pub event_name: String,
    format_id: u8,
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
        let (thread_name, thread_tid, thread_tgid, cpu, flags, timestamp, event_name, _) =
            extract_base_fields(&parts);
        Some(Self {
            thread_name,
            thread_tid,
            thread_tgid,
            cpu,
            flags,
            timestamp,
            event_name,
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
    #[pyo3(signature = (thread_name, thread_tid, thread_tgid, cpu, flags, timestamp, comm, pid, prio, group_dead))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thread_name: String,
        thread_tid: u32,
        thread_tgid: u32,
        cpu: u32,
        flags: String,
        timestamp: f64,
        comm: String,
        pid: u32,
        prio: i32,
        group_dead: bool,
    ) -> PyResult<Self> {
        validate_timestamp(timestamp)?;
        Ok(Self {
            thread_name,
            thread_tid,
            thread_tgid,
            cpu,
            flags,
            timestamp,
            event_name: Self::EVENT_NAME.to_string(),
            format_id: 0,
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

    #[getter]
    pub fn payload(&self) -> PyResult<String> {
        self.render_payload()
    }

    #[staticmethod]
    pub fn template() -> &'static str {
        Self::formats().template(0).unwrap().template_str()
    }

    pub fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.timestamp)?;
        let payload = self.payload()?;
        Ok(format_trace_header(
            &self.thread_name, self.thread_tid, self.thread_tgid, self.cpu,
            &self.flags, self.timestamp, &self.event_name,
            &payload
        ))
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "TraceSchedProcessExit(comm={:?}, pid={}, prio={}, group_dead={}, timestamp={})",
            self.comm, self.pid, self.prio, self.group_dead, self.timestamp
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

    #[getter]
    pub fn timestamp_ms(&self) -> f64 {
        self.timestamp * 1_000.0
    }

    #[setter]
    pub fn set_timestamp_ms(&mut self, value: f64) -> PyResult<()> {
        self.timestamp = validate_timestamp(value / 1_000.0)?;
        Ok(())
    }

    #[getter]
    pub fn timestamp_ns(&self) -> u64 {
        (self.timestamp * 1_000_000_000.0).round() as u64
    }

    #[setter]
    pub fn set_timestamp_ns(&mut self, value: u64) -> PyResult<()> {
        self.timestamp = (value as f64) / 1_000_000_000.0;
        Ok(())
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
        assert_eq!(trace.thread_name, "bash");
        assert_eq!(trace.thread_tid, 1977);
        assert_eq!(trace.thread_tgid, 12);
        assert_eq!(trace.cpu, 0);
        assert_eq!(
            trace.payload().expect("payload must work"),
            "comm=bash pid=1977 prio=120 group_dead=1"
        );
        assert_eq!(
            trace.to_string().expect("to_string must work"),
            "bash-1977 (12) [000] .... 12345.678901: sched_process_exit: comm=bash pid=1977 prio=120 group_dead=1"
        );
    }
}
