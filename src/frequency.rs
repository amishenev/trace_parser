use pyo3::prelude::*;
use regex::Captures;
use std::sync::LazyLock;

use crate::common::{
    cap_parse, cap_str, contains_any, parse_template_event, validate_timestamp, BaseTraceParts,
    EventType, FastMatch, TemplateEvent,
};
use crate::format_registry::{FormatRegistry, FormatSpec};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::trace::{extract_base_fields, format_trace_header};

static CPU_TEMPLATE: LazyLock<PayloadTemplate> = LazyLock::new(|| {
    PayloadTemplate::new(
        "state={state} cpu_id={cpu_id}",
        &[("state", FieldSpec::u32()), ("cpu_id", FieldSpec::u32())],
    )
});

static CPU_FORMATS: LazyLock<FormatRegistry> = LazyLock::new(|| {
    FormatRegistry::new(vec![
        FormatSpec {
            kind: 0,
            template: &CPU_TEMPLATE,
        },
    ])
});

static DEV_TEMPLATE: LazyLock<PayloadTemplate> = LazyLock::new(|| {
    PayloadTemplate::new(
        "clk={clk} state={state} cpu_id={cpu_id}",
        &[
            ("clk", FieldSpec::choice(&["ddr_devfreq", "l3c_devfreq"])),
            ("state", FieldSpec::u32()),
            ("cpu_id", FieldSpec::u32()),
        ],
    )
});

static DEV_FORMATS: LazyLock<FormatRegistry> = LazyLock::new(|| {
    FormatRegistry::new(vec![
        FormatSpec {
            kind: 0,
            template: &DEV_TEMPLATE,
        },
    ])
});

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct TraceCpuFrequency {
    #[pyo3(get)]
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
    pub state: u32,
    #[pyo3(get, set)]
    pub cpu_id: u32,
}

#[pyclass(skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct TraceDevFrequency {
    #[pyo3(get)]
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
    pub clk: String,
    #[pyo3(get, set)]
    pub state: u32,
    #[pyo3(get, set)]
    pub cpu_id: u32,
}

impl EventType for TraceCpuFrequency {
    const EVENT_NAME: &'static str = "cpu_frequency";
}

impl FastMatch for TraceCpuFrequency {}

impl TemplateEvent for TraceCpuFrequency {
    fn formats() -> &'static FormatRegistry {
        &CPU_FORMATS
    }

    fn parse_payload(
        parts: BaseTraceParts,
        captures: &Captures<'_>,
        _format_id: u8,
    ) -> Option<Self> {
        let (thread_name, thread_tid, thread_tgid, cpu, flags, timestamp, event_name, _) = extract_base_fields(&parts);
        Some(Self {
            thread_name,
            thread_tid,
            thread_tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            format_id: 0,
            state: cap_parse(captures, "state")?,
            cpu_id: cap_parse(captures, "cpu_id")?,
        })
    }

    fn render_payload(&self) -> PyResult<String> {
        let template = Self::formats().template(0).unwrap();
        let values: [(&str, Option<TemplateValue>); 2] = [
            ("state", Some(TemplateValue::U32(self.state))),
            ("cpu_id", Some(TemplateValue::U32(self.cpu_id))),
        ];
        Ok(template
            .format(&values)
            .expect("cpu_frequency template must render"))
    }
}

impl EventType for TraceDevFrequency {
    const EVENT_NAME: &'static str = "clock_set_rate";
}

impl FastMatch for TraceDevFrequency {
    fn payload_quick_check(line: &str) -> bool {
        contains_any(line, &["clk=ddr_devfreq", "clk=l3c_devfreq"])
    }
}

impl TemplateEvent for TraceDevFrequency {
    fn formats() -> &'static FormatRegistry {
        &DEV_FORMATS
    }

    fn parse_payload(
        parts: BaseTraceParts,
        captures: &Captures<'_>,
        _format_id: u8,
    ) -> Option<Self> {
        let (thread_name, thread_tid, thread_tgid, cpu, flags, timestamp, event_name, _) = extract_base_fields(&parts);
        Some(Self {
            thread_name,
            thread_tid,
            thread_tgid,
            cpu,
            flags,
            timestamp,
            event_name,
            format_id: 0,
            clk: cap_str(captures, "clk")?,
            state: cap_parse(captures, "state")?,
            cpu_id: cap_parse(captures, "cpu_id")?,
        })
    }

    fn render_payload(&self) -> PyResult<String> {
        let template = Self::formats().template(0).unwrap();
        let values: [(&str, Option<TemplateValue>); 3] = [
            ("clk", Some(TemplateValue::Str(&self.clk))),
            ("state", Some(TemplateValue::U32(self.state))),
            ("cpu_id", Some(TemplateValue::U32(self.cpu_id))),
        ];
        Ok(template
            .format(&values)
            .expect("clock_set_rate template must render"))
    }
}

#[pymethods]
impl TraceCpuFrequency {
    #[new]
    #[pyo3(signature = (thread_name, thread_tid, thread_tgid, cpu, flags, timestamp, state, cpu_id))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thread_name: String,
        thread_tid: u32,
        thread_tgid: u32,
        cpu: u32,
        flags: String,
        timestamp: f64,
        state: u32,
        cpu_id: u32,
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
            state,
            cpu_id,
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

    #[getter]
    pub fn template(&self) -> &'static str {
        Self::formats().template(self.format_id).unwrap().template_str()
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
            "TraceCpuFrequency(thread_name={:?}, thread_tid={}, state={}, cpu_id={})",
            self.thread_name, self.thread_tid, self.state, self.cpu_id
        ))
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.thread_name == other.thread_name
            && self.thread_tid == other.thread_tid
            && self.thread_tgid == other.thread_tgid
            && self.cpu == other.cpu
            && self.flags == other.flags
            && (self.timestamp - other.timestamp).abs() < 1e-9
            && self.event_name == other.event_name
            && self.state == other.state
            && self.cpu_id == other.cpu_id
    }

    fn __str__(&self) -> PyResult<String> {
        self.to_string()
    }

    fn __copy__(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<Self>> {
        slf.clone().into_pyobject(py).map(|o| o.unbind())
    }

    fn __deepcopy__(&self, py: Python<'_>, _memo: &Bound<'_, PyAny>) -> PyResult<Py<Self>> {
        self.clone().into_pyobject(py).map(|o| o.unbind())
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

#[pymethods]
impl TraceDevFrequency {
    #[new]
    #[pyo3(signature = (thread_name, thread_tid, thread_tgid, cpu, flags, timestamp, clk, state, cpu_id))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thread_name: String,
        thread_tid: u32,
        thread_tgid: u32,
        cpu: u32,
        flags: String,
        timestamp: f64,
        clk: String,
        state: u32,
        cpu_id: u32,
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
            clk,
            state,
            cpu_id,
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

    #[getter]
    pub fn template(&self) -> &'static str {
        Self::formats().template(self.format_id).unwrap().template_str()
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
            "TraceDevFrequency(thread_name={:?}, thread_tid={}, clk={:?}, state={}, cpu_id={})",
            self.thread_name, self.thread_tid, self.clk, self.state, self.cpu_id
        ))
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.thread_name == other.thread_name
            && self.thread_tid == other.thread_tid
            && self.thread_tgid == other.thread_tgid
            && self.cpu == other.cpu
            && self.flags == other.flags
            && (self.timestamp - other.timestamp).abs() < 1e-9
            && self.event_name == other.event_name
            && self.clk == other.clk
            && self.state == other.state
            && self.cpu_id == other.cpu_id
    }

    fn __str__(&self) -> PyResult<String> {
        self.to_string()
    }

    fn __copy__(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<Self>> {
        slf.clone().into_pyobject(py).map(|o| o.unbind())
    }

    fn __deepcopy__(&self, py: Python<'_>, _memo: &Bound<'_, PyAny>) -> PyResult<Py<Self>> {
        self.clone().into_pyobject(py).map(|o| o.unbind())
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
    use crate::{TraceCpuFrequency, TraceDevFrequency};

    #[test]
    fn cpu_frequency_parses() {
        let line = "swapper-0 (0) [000] .... 12345.678900: cpu_frequency: state=933000000 cpu_id=0";
        let trace = TraceCpuFrequency::parse(line).expect("cpu_frequency must parse");
        assert_eq!(trace.thread_name, "swapper");
        assert_eq!(trace.thread_tid, 0);
        assert_eq!(trace.thread_tgid, 0);
        assert_eq!(trace.cpu, 0);
        assert_eq!(trace.flags, "....");
        assert!((trace.timestamp - 12345.678900).abs() < 1e-9);
        assert_eq!(trace.event_name, "cpu_frequency");
        assert_eq!(trace.state, 933000000);
        assert_eq!(trace.cpu_id, 0);
    }

    #[test]
    fn cpu_frequency_payload_and_template() {
        let line = "swapper-0 (0) [000] .... 12345.678900: cpu_frequency: state=933000000 cpu_id=0";
        let trace = TraceCpuFrequency::parse(line).expect("cpu_frequency must parse");
        assert_eq!(trace.payload().unwrap(), "state=933000000 cpu_id=0");
        assert_eq!(trace.template(), "state={state} cpu_id={cpu_id}");
    }

    #[test]
    fn dev_frequency_parses() {
        let line = "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: clk=ddr_devfreq state=933000000 cpu_id=0";
        let trace = TraceDevFrequency::parse(line).expect("clock_set_rate must parse");
        assert_eq!(trace.thread_name, "swapper");
        assert_eq!(trace.thread_tid, 0);
        assert_eq!(trace.thread_tgid, 0);
        assert_eq!(trace.cpu, 0);
        assert_eq!(trace.flags, "....");
        assert!((trace.timestamp - 12345.678900).abs() < 1e-9);
        assert_eq!(trace.event_name, "clock_set_rate");
        assert_eq!(trace.clk, "ddr_devfreq");
        assert_eq!(trace.state, 933000000);
        assert_eq!(trace.cpu_id, 0);
    }

    #[test]
    fn dev_frequency_payload_and_template() {
        let line = "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: clk=ddr_devfreq state=933000000 cpu_id=0";
        let trace = TraceDevFrequency::parse(line).expect("clock_set_rate must parse");
        assert_eq!(trace.payload().unwrap(), "clk=ddr_devfreq state=933000000 cpu_id=0");
        assert_eq!(trace.template(), "clk={clk} state={state} cpu_id={cpu_id}");
    }

    #[test]
    fn cpu_frequency_to_string() {
        let line = "swapper-0 (0) [000] .... 12345.678900: cpu_frequency: state=933000000 cpu_id=0";
        let trace = TraceCpuFrequency::parse(line).expect("cpu_frequency must parse");
        let output = trace.to_string().unwrap();
        assert_eq!(output, line);
    }

    #[test]
    fn dev_frequency_to_string() {
        let line = "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: clk=ddr_devfreq state=933000000 cpu_id=0";
        let trace = TraceDevFrequency::parse(line).expect("clock_set_rate must parse");
        let output = trace.to_string().unwrap();
        assert_eq!(output, line);
    }
}
