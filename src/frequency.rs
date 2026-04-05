use trace_parser_macros::trace_event_class;

#[trace_event_class]
#[trace_event(name = "cpu_frequency")]
#[define_template("state={state} cpu_id={cpu_id}")]
pub struct TraceCpuFrequency {
    #[field(private)]
    format_id: u8,
    #[field]
    pub thread_name: String,
    #[field]
    pub thread_tid: u32,
    #[field]
    pub thread_tgid: Option<u32>,
    #[field]
    pub cpu: u32,
    #[field]
    pub flags: String,
    #[field]
    pub timestamp: f64,
    #[field(readonly)]
    pub event_name: String,
    #[field]
    pub state: u32,
    #[field]
    pub cpu_id: u32,
}

#[trace_event_class]
#[trace_event(name = "clock_set_rate")]
#[fast_match(contains_any = ["clk=ddr_devfreq", "clk=l3c_devfreq"])]
#[define_template("clk={clk} state={state} cpu_id={cpu_id}")]
pub struct TraceDevFrequency {
    #[field(private)]
    format_id: u8,
    #[field]
    pub thread_name: String,
    #[field]
    pub thread_tid: u32,
    #[field]
    pub thread_tgid: Option<u32>,
    #[field]
    pub cpu: u32,
    #[field]
    pub flags: String,
    #[field]
    pub timestamp: f64,
    #[field(readonly)]
    pub event_name: String,
    #[field(choice = ["ddr_devfreq", "l3c_devfreq"])]
    pub clk: String,
    #[field]
    pub state: u32,
    #[field]
    pub cpu_id: u32,
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
        assert_eq!(trace.thread_tgid, Some(0));
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
        assert_eq!(trace.thread_tgid, Some(0));
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
        assert_eq!(
            trace.payload().unwrap(),
            "clk=ddr_devfreq state=933000000 cpu_id=0"
        );
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
