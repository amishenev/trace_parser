"""Smoke tests for TraceExit with dual event names (exit1, exit2)."""

from trace_parser import TraceExit, parse_trace


def test_trace_exit_exit1_parse():
    """Test parsing exit1 event."""
    line = "task-100 (100) [000] .... 123.456789: exit1: pid=123 comm=test tgid=100"
    exit_event = TraceExit.parse(line)
    assert exit_event is not None
    assert exit_event.pid == 123
    assert exit_event.comm == "test"
    assert exit_event.exit_tgid == 100


def test_trace_exit_exit2_parse():
    """Test parsing exit2 event."""
    line = "task-200 (200) [001] .... 456.789012: exit2: pid=456 comm=foo tgid=200"
    exit_event = TraceExit.parse(line)
    assert exit_event is not None
    assert exit_event.pid == 456
    assert exit_event.comm == "foo"
    assert exit_event.exit_tgid == 200


def test_trace_exit_to_string():
    """Test round-trip via to_string()."""
    line = "task-100 (100) [000] .... 123.456789: exit1: pid=123 comm=test tgid=100"
    exit_event = TraceExit.parse(line)
    assert exit_event.to_string() == line


def test_trace_exit_payload():
    """Test payload getter."""
    line = "task-100 (100) [000] .... 123.456789: exit1: pid=123 comm=test tgid=100"
    exit_event = TraceExit.parse(line)
    assert exit_event.payload == "pid=123 comm=test tgid=100"


def test_trace_exit_template():
    """Test template getter."""
    exit_event = TraceExit.parse(
        "task-100 (100) [000] .... 123.456789: exit1: pid=123 comm=test tgid=100"
    )
    assert exit_event.template == "pid={pid} comm={comm} tgid={tgid}"


def test_parse_trace_exit1():
    """Test factory parser with exit1."""
    line = "task-100 (100) [000] .... 123.456789: exit1: pid=123 comm=test tgid=100"
    event = parse_trace(line)
    assert event is not None
    assert isinstance(event, TraceExit)
    assert event.pid == 123


def test_parse_trace_exit2():
    """Test factory parser with exit2."""
    line = "task-200 (200) [001] .... 456.789012: exit2: pid=456 comm=foo tgid=200"
    event = parse_trace(line)
    assert event is not None
    assert isinstance(event, TraceExit)
    assert event.pid == 456
