"""Benchmark tests using pytest-benchmark."""

from benches.trace_pure_py import SAMPLE_LINE, parse_trace_regex, parse_trace_string


def test_parse_rust(benchmark):
    """Benchmark Rust (PyO3) Trace.parse."""
    from trace_parser import Trace

    result = benchmark(Trace.parse, SAMPLE_LINE)
    assert result is not None
    assert result.thread_name == "bash"
    assert result.thread_tid == 1977


def test_parse_py_string(benchmark):
    """Benchmark pure Python string methods parser."""
    result = benchmark(parse_trace_string, SAMPLE_LINE)
    assert result is not None
    assert result.thread_name == "bash"
    assert result.thread_tid == 1977


def test_parse_py_regex(benchmark):
    """Benchmark pure Python regex parser."""
    result = benchmark(parse_trace_regex, SAMPLE_LINE)
    assert result is not None
    assert result.thread_name == "bash"
    assert result.thread_tid == 1977
