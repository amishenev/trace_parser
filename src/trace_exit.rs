use pyo3::prelude::*;
use regex::Captures;
use std::sync::LazyLock;
use lexical_core::parse;

use crate::common::{validate_timestamp, BaseTraceParts, EventType, FastMatch, TemplateEvent};
use crate::format_registry::{FormatRegistry, FormatSpec};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::trace::{format_trace_header, extract_base_fields};

static TEMPLATE: LazyLock<PayloadTemplate> = LazyLock::new(|| {
    PayloadTemplate::new(
        "pid={pid} comm={comm} tgid={tgid}",
        &[
            ("pid", FieldSpec::u32()),
            ("comm", FieldSpec::string()),
            ("tgid", FieldSpec::u32()),
        ]
    )
});

static FORMATS: LazyLock<FormatRegistry> = LazyLock::new(|| {
    FormatRegistry::new(vec![
        FormatSpec { kind: 0, template: &TEMPLATE }
    ])
});

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct TraceExit {
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
    pub pid: u32,
    #[pyo3(get, set)]
    pub comm: String,
    #[pyo3(get, set)]
    pub tgid: u32,
}

impl EventType for TraceExit {
    const EVENT_NAME: &'static str = "exit1";
    const EVENT_ALIASES: &'static [&'static str] = &["exit2"];
}

impl FastMatch for TraceExit {}

impl TemplateEvent for TraceExit {
    fn formats() -> &'static FormatRegistry {
        &FORMATS
    }

    fn detect_format(_payload: &str) -> u8 {
        0
    }

    fn parse_payload(
        parts: BaseTraceParts,
        captures: &Captures<'_>,
        _format_id: u8,
    ) -> Option<Self> {
        let (thread_name, thread_tid, thread_tgid, cpu, flags, timestamp, event_name, _payload_raw) = extract_base_fields(&parts);
        Some(Self {
            thread_name,
            thread_tid,
            thread_tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            format_id: 0,
            pid: parse(captures.name("pid")?.as_str().as_bytes()).ok()?,
            comm: captures.name("comm")?.as_str().to_string(),
            tgid: parse(captures.name("tgid")?.as_str().as_bytes()).ok()?,
        })
    }

    fn render_payload(&self) -> PyResult<String> {
        let template = Self::formats().template(0).unwrap();
        let values: [(&str, Option<TemplateValue>); 3] = [
            ("pid", Some(TemplateValue::U32(self.pid))),
            ("comm", Some(TemplateValue::Str(&self.comm))),
            ("tgid", Some(TemplateValue::U32(self.tgid))),
        ];
        Ok(template.format(&values).expect("exit template must render"))
    }
}

#[pymethods]
impl TraceExit {
    #[new]
    #[pyo3(signature = (thread_name, thread_tid, thread_tgid, cpu, flags, timestamp, pid, comm, tgid))]
    fn new(
        thread_name: String,
        thread_tid: u32,
        thread_tgid: u32,
        cpu: u32,
        flags: String,
        timestamp: f64,
        pid: u32,
        comm: String,
        tgid: u32,
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
            pid,
            comm,
            tgid,
        })
    }

    #[getter]
    pub fn payload(&self) -> PyResult<String> {
        self.render_payload()
    }

    #[getter]
    pub fn template(&self) -> &'static str {
        Self::formats().template(self.format_id).unwrap().template_str()
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "TraceExit(pid={}, comm={:?}, tgid={})",
            self.pid, self.comm, self.tgid
        ))
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.thread_name == other.thread_name
            && self.thread_tid == other.thread_tid
            && self.tgid == other.tgid
            && self.cpu == other.cpu
            && self.flags == other.flags
            && self.timestamp == other.timestamp
            && self.event_name == other.event_name
            && self.pid == other.pid
            && self.comm == other.comm
            && self.tgid == other.tgid
    }

    fn __str__(&self) -> PyResult<String> {
        self.to_string()
    }

    fn __copy__(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(slf.into_pyobject(py).map(|o| o.into_any().unbind())?)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(self.clone())
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

    fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.timestamp)?;
        let payload = self.payload()?;
        Ok(format_trace_header(
            &self.thread_name, self.thread_tid, self.tgid, self.cpu,
            &self.flags, self.timestamp, &self.event_name,
            &payload
        ))
    }
}

// Helper функция для парсинга
fn parse_template_event<T>(line: &str) -> Option<T>
where
    T: TemplateEvent + Clone,
{
    let parts = BaseTraceParts::parse(line)?;
    let payload = parts.payload_raw.clone();
    let format_id = T::detect_format(&payload);
    let template = T::formats().template(format_id)?;
    let captures = template.captures(&payload)?;
    T::parse_payload(parts, &captures, format_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit1_parse() {
        let line = "task-100 (100) [000] .... 123.456789: exit1: pid=123 comm=test tgid=100";
        let exit = TraceExit::parse(line).expect("exit1 must parse");
        assert_eq!(exit.pid, 123);
        assert_eq!(exit.comm, "test");
        assert_eq!(exit.tgid, 100);
    }

    #[test]
    fn test_exit2_parse() {
        let line = "task-200 (200) [001] .... 456.789012: exit2: pid=456 comm=foo tgid=200";
        let exit = TraceExit::parse(line).expect("exit2 must parse");
        assert_eq!(exit.pid, 456);
        assert_eq!(exit.comm, "foo");
        assert_eq!(exit.tgid, 200);
    }

    #[test]
    fn test_exit_to_string() {
        let line = "task-100 (100) [000] .... 123.456789: exit1: pid=123 comm=test tgid=100";
        let exit = TraceExit::parse(line).expect("exit1 must parse");
        let result = exit.to_string().expect("to_string must work");
        assert_eq!(result, line);
    }
}
