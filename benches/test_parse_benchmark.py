"""Benchmarks comparing Python line-by-line vs Rust parse_trace_file()."""

from __future__ import annotations

import pytest
from trace_parser import parse_trace, parse_trace_file


@pytest.fixture(scope="module")
def test_trace_file(tmp_path_factory: pytest.TempPathFactory) -> str:
    """Generate a test trace file with 10K lines for quick benchmarks."""
    from generate_test_trace import generate_trace_file

    tmp_dir = tmp_path_factory.mktemp("traces")
    path = tmp_dir / "test_trace_10k.trace"
    generate_trace_file(str(path), lines=10000, seed=42)
    return str(path)


def test_python_line_by_line(benchmark, test_trace_file: str) -> None:
    """Python: parse_trace() for each line.

    This is the slowest approach - FFI call for every line.
    """

    def fn():
        with open(test_trace_file) as f:
            for line in f:
                parse_trace(line)

    benchmark(fn)


def test_rust_parse_file(benchmark, test_trace_file: str) -> None:
    """Rust: parse_trace_file() single call.

    This is the fastest approach - single FFI call, all parsing in Rust.
    """

    def fn():
        parse_trace_file(test_trace_file)

    benchmark(fn)


def test_python_with_filter(benchmark, test_trace_file: str) -> None:
    """Python: line-by-line with filtering.

    Filter in Python, then parse matching lines.
    """

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

    def fn():
        parse_trace_file(test_trace_file, filter_event="sched_switch")

    benchmark(fn)
