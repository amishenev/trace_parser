use std::collections::HashMap;

use once_cell::sync::Lazy;
use pyo3::prelude::*;

use crate::common::{parse_base_parts, validate_timestamp};
use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};
use crate::trace::Trace;

static CPU_FREQUENCY_TEMPLATE: Lazy<PayloadTemplate> = Lazy::new(|| {
    PayloadTemplate::new(
        "state={state} cpu_id={cpu_id}",
        &[("state", FieldSpec::u32()), ("cpu_id", FieldSpec::u32())],
    )
});

static CLOCK_SET_RATE_TEMPLATE: Lazy<PayloadTemplate> = Lazy::new(|| {
    PayloadTemplate::new(
        "clk={clk} state={state} cpu_id={cpu_id}",
        &[
            ("clk", FieldSpec::string()),
            ("state", FieldSpec::u32()),
            ("cpu_id", FieldSpec::u32()),
        ],
    )
});

static DEV_FREQUENCY_TEMPLATE: Lazy<PayloadTemplate> = Lazy::new(|| {
    PayloadTemplate::new(
        "clk={clk} state={state} cpu_id={cpu_id}",
        &[
            ("clk", FieldSpec::choice(&["ddr_devfreq", "l3c_devfreq"])),
            ("state", FieldSpec::u32()),
            ("cpu_id", FieldSpec::u32()),
        ],
    )
});

#[pyclass]
#[derive(Clone, Debug)]
pub struct TraceCpuFrequency {
    #[pyo3(get)]
    pub(crate) base: Trace,
    #[pyo3(get, set)]
    pub(crate) format_id: String,
    #[pyo3(get, set)]
    pub(crate) state: u32,
    #[pyo3(get, set)]
    pub(crate) cpu_id: u32,
}

#[pymethods]
impl TraceCpuFrequency {
    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        let Some(parts) = parse_base_parts(line) else {
            return false;
        };
        parts.event_name == "cpu_frequency" && CPU_FREQUENCY_TEMPLATE.is_match(&parts.payload_raw)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        let parts = parse_base_parts(line)?;
        if parts.event_name != "cpu_frequency" {
            return None;
        }
        let captures = CPU_FREQUENCY_TEMPLATE.captures(&parts.payload_raw)?;
        let state = captures.name("state")?.as_str().parse().ok()?;
        let cpu_id = captures.name("cpu_id")?.as_str().parse().ok()?;
        Some(Self {
            base: Trace::from_parts(parts),
            format_id: "default".to_owned(),
            state,
            cpu_id,
        })
    }

    pub(crate) fn payload_to_string(&self) -> PyResult<String> {
        let values = HashMap::from([
            ("state", TemplateValue::U32(self.state)),
            ("cpu_id", TemplateValue::U32(self.cpu_id)),
        ]);
        Ok(CPU_FREQUENCY_TEMPLATE
            .format(&values)
            .expect("cpu_frequency template must render"))
    }

    pub(crate) fn to_string(&self) -> PyResult<String> {
        validate_timestamp(self.base.timestamp)?;
        Ok(self.base.to_string_with_payload(&self.payload_to_string()?))
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct TraceDevFrequency {
    #[pyo3(get)]
    pub(crate) base: Trace,
    #[pyo3(get, set)]
    pub(crate) format_id: String,
    #[pyo3(get, set)]
    pub(crate) clk: String,
    #[pyo3(get, set)]
    pub(crate) state: u32,
    #[pyo3(get, set)]
    pub(crate) cpu_id: u32,
}

#[pymethods]
impl TraceDevFrequency {
    #[staticmethod]
    pub fn can_be_parsed(line: &str) -> bool {
        let Some(parts) = parse_base_parts(line) else {
            return false;
        };
        parts.event_name == "clock_set_rate" && DEV_FREQUENCY_TEMPLATE.is_match(&parts.payload_raw)
    }

    #[staticmethod]
    pub fn parse(line: &str) -> Option<Self> {
        let parts = parse_base_parts(line)?;
        if parts.event_name != "clock_set_rate" {
            return None;
        }
        let captures = DEV_FREQUENCY_TEMPLATE.captures(&parts.payload_raw)?;
        let clk = captures.name("clk")?.as_str().to_owned();
        let state = captures.name("state")?.as_str().parse().ok()?;
        let cpu_id = captures.name("cpu_id")?.as_str().parse().ok()?;
        Some(Self {
            base: Trace::from_parts(parts),
            format_id: "default".to_owned(),
            clk,
            state,
            cpu_id,
        })
    }

    pub(crate) fn payload_to_string(&self) -> PyResult<String> {
        let values = HashMap::from([
            ("clk", TemplateValue::Str(&self.clk)),
            ("state", TemplateValue::U32(self.state)),
            ("cpu_id", TemplateValue::U32(self.cpu_id)),
        ]);
        Ok(CLOCK_SET_RATE_TEMPLATE
            .format(&values)
            .expect("clock_set_rate template must render"))
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
        let line = "swapper-0 (0) [000] .... 12345.678900: cpu_frequency: state=1200000 cpu_id=0";
        let trace = TraceCpuFrequency::parse(line).expect("cpu_frequency must parse");
        assert_eq!(trace.state, 1_200_000);
        assert_eq!(trace.cpu_id, 0);
        assert_eq!(
            trace.payload_to_string().expect("payload_to_string must work"),
            "state=1200000 cpu_id=0"
        );
        assert_eq!(
            trace.to_string().expect("to_string must work"),
            "swapper-0 (0) [000] .... 12345.678900: cpu_frequency: state=1200000 cpu_id=0"
        );
    }

    #[test]
    fn dev_frequency_parses_ddr_devfreq() {
        let line =
            "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: clk=ddr_devfreq state=933000000 cpu_id=0";
        let trace = TraceDevFrequency::parse(line).expect("ddr_devfreq must parse");
        assert_eq!(trace.clk, "ddr_devfreq");
        assert_eq!(trace.state, 933_000_000);
        assert_eq!(
            trace.payload_to_string().expect("payload_to_string must work"),
            "clk=ddr_devfreq state=933000000 cpu_id=0"
        );
        assert_eq!(
            trace.to_string().expect("to_string must work"),
            "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: clk=ddr_devfreq state=933000000 cpu_id=0"
        );
    }

    #[test]
    fn dev_frequency_parses_l3c_devfreq() {
        let line =
            "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: clk=l3c_devfreq state=600000000 cpu_id=0";
        let trace = TraceDevFrequency::parse(line).expect("l3c_devfreq must parse");
        assert_eq!(trace.clk, "l3c_devfreq");
        assert_eq!(trace.state, 600_000_000);
    }

    #[test]
    fn dev_frequency_rejects_other_clock_names() {
        let line =
            "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: clk=gpu_clk state=800000000 cpu_id=0";
        assert!(TraceDevFrequency::parse(line).is_none());
    }
}
