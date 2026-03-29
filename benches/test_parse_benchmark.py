"""Benchmarks comparing Python line-by-line vs Rust parse_trace_file()."""

from __future__ import annotations

import tempfile
import pytest
from trace_parser import parse_trace, parse_trace_file


def generate_test_trace(path: str, lines: int = 10000) -> None:
    """Generate a test trace file with mixed events."""
    import random
    random.seed(42)
    
    events = [
        "sched_switch",
        "sched_wakeup",
        "sched_process_exit",
        "clock_set_rate",
    ]
    
    with open(path, "w") as f:
        for i in range(lines):
            event = random.choice(events)
            timestamp = 12345.678900 + i * 0.000001
            cpu = i % 4
            tid = 1000 + i % 100
            tgid = 1000 + i % 50
            
            if event == "sched_switch":
                f.write(
                    f"bash-{tid} ({tgid}) [{cpu:03d}] .... {timestamp:.6f}: sched_switch: "
                    f"prev_comm=bash prev_pid={tid} prev_prio=120 prev_state=S ==> "
                    f"next_comm=worker next_pid={tid + 1} next_prio=120\n"
                )
            elif event == "sched_wakeup":
                f.write(
                    f"kworker-{tid} ({tgid}) [{cpu:03d}] .... {timestamp:.6f}: sched_wakeup: "
                    f"comm=bash pid={tid} prio=120 target_cpu={cpu:03d}\n"
                )
            elif event == "sched_process_exit":
                f.write(
                    f"bash-{tid} ({tgid}) [{cpu:03d}] .... {timestamp:.6f}: sched_process_exit: "
                    f"comm=bash pid={tid} prio=120 group_dead=0\n"
                )
            elif event == "clock_set_rate":
                clk = random.choice(["ddr_devfreq", "l3c_devfreq"])
                state = random.choice([600000000, 933000000, 1066000000])
                f.write(
                    f"swapper-0 (0) [{cpu:03d}] .... {timestamp:.6f}: clock_set_rate: "
                    f"clk={clk} state={state} cpu_id={cpu}\n"
                )


@pytest.fixture(scope="module")
def test_trace_file(tmp_path_factory: pytest.TempPathFactory) -> str:
    """Generate a test trace file with 1K lines for quick benchmarks."""
    tmp_dir = tmp_path_factory.mktemp("traces")
    path = tmp_dir / "test_trace_1k.trace"
    generate_test_trace(str(path), lines=1000)
    return str(path)


def test_python_line_by_line(benchmark, test_trace_file: str) -> None:
    """Python: parse_trace() for each line.

    This is the slowest approach - FFI call for every line.
    """
    # Warm up
    with open(test_trace_file) as f:
        for i, line in enumerate(f):
            if i >= 100:
                break
            parse_trace(line)

    def fn():
        with open(test_trace_file) as f:
            for line in f:
                parse_trace(line)

    benchmark(fn)


def test_rust_parse_file(benchmark, test_trace_file: str) -> None:
    """Rust: parse_trace_file() single call.

    This is the fastest approach - single FFI call, all parsing in Rust.
    """
    # Warm up
    parse_trace_file(test_trace_file)

    def fn():
        parse_trace_file(test_trace_file)

    benchmark(fn)


def test_python_with_filter(benchmark, test_trace_file: str) -> None:
    """Python: line-by-line with filtering.

    Filter in Python, then parse matching lines.
    """
    # Warm up
    with open(test_trace_file) as f:
        for i, line in enumerate(f):
            if i >= 100:
                break
            if "sched_switch" in line:
                parse_trace(line)

    def fn():
        with open(test_trace_file) as f:
            for line in f:
                if "sched_switch" in line:
                    parse_trace(line)

    benchmark(fn)


def test_rust_parse_file_with_filter(benchmark, test_trace_file: str) -> None:
    """Rust: parse_trace_file() with filter_event.

    Filter in Rust - faster than filtering in Python.
    """
    # Warm up
    parse_trace_file(test_trace_file, filter_event="sched_switch")

    def fn():
        parse_trace_file(test_trace_file, filter_event="sched_switch")

    benchmark(fn)
