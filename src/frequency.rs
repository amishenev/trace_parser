use std::collections::HashMap;

use once_cell::sync::Lazy;
use pyo3::prelude::*;
use regex::Captures;

use crate::common::{
    cap_parse, cap_str, contains_any, parse_template_event, validate_timestamp, BaseTraceParts,
    EventType, FastMatch, TemplateEvent,
};
use crate::format_registry::{FormatRegistry, FormatSpec};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::trace::Trace;

static CPU_TEMPLATE: Lazy<PayloadTemplate> = Lazy::new(|| {
    PayloadTemplate::new(
        "state={state} cpu_id={cpu_id}",
        &[("state", FieldSpec::u32()), ("cpu_id", FieldSpec::u32())],
    )
});

static CPU_FORMATS: Lazy<FormatRegistry> = Lazy::new(|| {
    FormatRegistry::new(vec![
        FormatSpec {
            kind: 0,
            template: &CPU_TEMPLATE,
        },
    ])
});

static DEV_TEMPLATE: Lazy<PayloadTemplate> = Lazy::new(|| {
    PayloadTemplate::new(
        "clk={clk} state={state} cpu_id={cpu_id}",
        &[
            ("clk", FieldSpec::choice(&["ddr_devfreq", "l3c_devfreq"])),
            ("state", FieldSpec::u32()),
            ("cpu_id", FieldSpec::u32()),
        ],
    )
});

static DEV_FORMATS: Lazy<FormatRegistry> = Lazy::new(|| {
    FormatRegistry::new(vec![
        FormatSpec {
            kind: 0,
            template: &DEV_TEMPLATE,
        },
    ])
});

#[pyclass]
#[derive(Clone, Debug)]
pub struct TraceCpuFrequency {
    #[pyo3(get)]
    pub(crate) base: Trace,
    #[pyo3(get, set)]
    pub(crate) format_id: u8,
    #[pyo3(get, set)]
    pub(crate) state: u32,
    #[pyo3(get, set)]
    pub(crate) cpu_id: u32,
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct TraceDevFrequency {
    #[pyo3(get)]
    pub(crate) base: Trace,
    #[pyo3(get, set)]
    pub(crate) format_id: u8,
    #[pyo3(get, set)]
    pub(crate) clk: String,
    #[pyo3(get, set)]
    pub(crate) state: u32,
    #[pyo3(get, set)]
    pub(crate) cpu_id: u32,
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
        Some(Self {
            base: Trace::from_parts(parts),
            format_id: 0,
            state: cap_parse(captures, "state")?,
            cpu_id: cap_parse(captures, "cpu_id")?,
        })
    }

    fn render_payload(&self) -> PyResult<String> {
        let template = Self::formats().template(0).unwrap();
        let values = HashMap::from([
            ("state", TemplateValue::U32(self.state)),
            ("cpu_id", TemplateValue::U32(self.cpu_id)),
        ]);
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
        Some(Self {
            base: Trace::from_parts(parts),
            format_id: 0,
            clk: cap_str(captures, "clk")?,
            state: cap_parse(captures, "state")?,
            cpu_id: cap_parse(captures, "cpu_id")?,
        })
    }

    fn render_payload(&self) -> PyResult<String> {
        let template = Self::formats().template(0).unwrap();
        let values = HashMap::from([
            ("clk", TemplateValue::Str(&self.clk)),
            ("state", TemplateValue::U32(self.state)),
            ("cpu_id", TemplateValue::U32(self.cpu_id)),
        ]);
        Ok(template
            .format(&values)
            .expect("clock_set_rate template must render"))
    }
}

#[pymethods]
impl TraceCpuFrequency {
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

    pub(crate) fn payload_to_string(&self) -> PyResult<String> {
        self.render_payload()
    }

    pub(crate) fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.base.timestamp)?;
        Ok(self.base.to_string_with_payload(&self.payload_to_string()?))
    }
}

#[pymethods]
impl TraceDevFrequency {
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

    pub(crate) fn payload_to_string(&self) -> PyResult<String> {
        self.render_payload()
    }

    pub(crate) fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.base.timestamp)?;
        Ok(self.base.to_string_with_payload(&self.payload_to_string()?))
    }
}

#[cfg(test)]
mod tests {
    use crate::{TraceCpuFrequency, TraceDevFrequency};

    #[test]
    fn cpu_frequency_parses() {
        let line = "swapper-0 (0) [000] .... 12345.678900: cpu_frequency: state=933000000 cpu_id=0";
        let trace = TraceCpuFrequency::parse(line).expect("cpu_frequency must parse");
        assert_eq!(trace.state, 933000000);
        assert_eq!(trace.cpu_id, 0);
    }

    #[test]
    fn dev_frequency_parses() {
        let line = "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: clk=ddr_devfreq state=933000000 cpu_id=0";
        let trace = TraceDevFrequency::parse(line).expect("clock_set_rate must parse");
        assert_eq!(trace.clk, "ddr_devfreq");
        assert_eq!(trace.state, 933000000);
        assert_eq!(trace.cpu_id, 0);
    }
}
