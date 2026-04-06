#!/usr/bin/env python3
"""Pure Python trace line parser benchmarks.

Compares:
- Rust (PyO3) Trace.parse
- Pure Python string methods
- Pure Python regex
"""

from __future__ import annotations

import re
import timeit
from dataclasses import dataclass
from typing import Optional

# ---------------------------------------------------------------------------
# Single dataclass for parsed trace events
# ---------------------------------------------------------------------------


@dataclass(slots=True)
class TraceEvent:
    """Parsed trace event."""

    thread_name: str
    thread_tid: int
    thread_tgid: Optional[int]
    cpu: int
    flags: str
    timestamp: float
    event_name: str
    payload_raw: str


# ---------------------------------------------------------------------------
# Pure Python: string methods (EAFP)
# ---------------------------------------------------------------------------


def parse_trace_string(line: str) -> Optional[TraceEvent]:
    """Parse trace line using rsplit + try/except."""
    pos = 0
    while True:
        colon = line.find(": ", pos)
        if colon < 0:
            return None

        try:
            base = line[:colon]
            event_payload = line[colon + 2 :]

            # "TASK-TID (TGID) [CPU] FLAGS TIMESTAMP"
            base, cpu, flags, timestamp_str = base.rsplit(maxsplit=3)

            # TIMESTAMP
            timestamp = float(timestamp_str)

            # FLAGS — must be exactly 4 chars
            assert len(flags) == 4

            # CPU — must be [NNN]
            assert cpu.startswith("[") and cpu.endswith("]")
            cpu = int(cpu.strip("[]"))

            # base must end with ")"
            assert base.endswith(")")

            # Split TGID
            base, tgid_str = base[:-1].rsplit("(", 1)
            tgid_str = tgid_str.strip()
            thread_tgid: Optional[int]
            if not tgid_str or set(tgid_str) == {"-"}:
                thread_tgid = None
            else:
                thread_tgid = int(tgid_str)

            # thread_name-TID
            thread_name, tid_str = base.strip().rsplit("-", 1)
            thread_tid = int(tid_str)

            # event_name: payload
            event_name, payload_raw = event_payload.split(": ", 1)
            event_name = event_name.strip()

            return TraceEvent(
                thread_name=thread_name,
                thread_tid=thread_tid,
                thread_tgid=thread_tgid,
                cpu=cpu,
                flags=flags,
                timestamp=timestamp,
                event_name=event_name,
                payload_raw=payload_raw,
            )

        except (ValueError, AssertionError, IndexError):
            pos = colon + 2
            continue


# ---------------------------------------------------------------------------
# Pure Python: regex
# ---------------------------------------------------------------------------

_TRACE_RE = re.compile(
    r"^(?P<thread_name>.+?)-"
    r"(?P<thread_tid>\d+)\s*"
    r"\((?P<thread_tgid>.*?)\)\s*"
    r"\[(?P<cpu>\d+)\]\s*"
    r"(?P<flags>[a-z0-9.\-]+)\s+"
    r"(?P<timestamp>\d+\.\d+):\s*"
    r"(?P<event_name>[^:]+?):\s*"
    r"(?P<payload_raw>.*)$"
)


def parse_trace_regex(line: str) -> Optional[TraceEvent]:
    """Parse trace line using compiled regex."""
    m = _TRACE_RE.match(line)
    if not m:
        return None
    tgid_str = m.group("thread_tgid").strip()
    if tgid_str == "-----" or not tgid_str:
        thread_tgid: Optional[int] = None
    else:
        try:
            thread_tgid = int(tgid_str)
        except ValueError:
            thread_tgid = None
    return TraceEvent(
        thread_name=m.group("thread_name"),
        thread_tid=int(m.group("thread_tid")),
        thread_tgid=thread_tgid,
        cpu=int(m.group("cpu")),
        flags=m.group("flags"),
        timestamp=float(m.group("timestamp")),
        event_name=m.group("event_name").strip(),
        payload_raw=m.group("payload_raw"),
    )


# ---------------------------------------------------------------------------
# Benchmark runner
# ---------------------------------------------------------------------------

SAMPLE_LINE = (
    "bash-1977 (12) [000] .... 12345.678901: sched_switch: "
    "prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> "
    "next_comm=worker next_pid=123 next_prio=120"
)


def bench_all(n: int = 100000) -> dict:
    """Run all benchmarks and return results."""
    results = {}

    # Rust PyO3
    try:
        from trace_parser import Trace

        def rust_fn():
            return Trace.parse(SAMPLE_LINE)

        # Warmup
        for _ in range(100):
            rust_fn()

        t_rust = timeit.timeit(rust_fn, number=n)
        results["rust"] = {
            "total_sec": round(t_rust, 3),
            "ns_per_line": round(t_rust / n * 1e9, 1),
            "lines_per_sec": round(n / t_rust, 1),
        }
    except Exception as e:
        print(f"Rust benchmark error: {e}")
        results["rust"] = None

    # Python string methods
    def py_str_fn():
        return parse_trace_string(SAMPLE_LINE)

    for _ in range(100):
        py_str_fn()
    t_str = timeit.timeit(py_str_fn, number=n)
    results["py_string"] = {
        "total_sec": round(t_str, 3),
        "ns_per_line": round(t_str / n * 1e9, 1),
        "lines_per_sec": round(n / t_str, 1),
    }

    # Python regex
    def py_re_fn():
        return parse_trace_regex(SAMPLE_LINE)

    for _ in range(100):
        py_re_fn()
    t_re = timeit.timeit(py_re_fn, number=n)
    results["py_regex"] = {
        "total_sec": round(t_re, 3),
        "ns_per_line": round(t_re / n * 1e9, 1),
        "lines_per_sec": round(n / t_re, 1),
    }

    return results


if __name__ == "__main__":
    import json

    n = 100000
    print(f"Running benchmarks ({n} iterations each)...")
    results = bench_all(n)

    print(f"\n{'Variant':<20} {'µs/line':>10} {'lines/sec':>12} {'vs Rust':>10}")
    print("-" * 54)

    rust_lps = results["rust"]["lines_per_sec"] if results["rust"] else None

    for name, key in [
        ("Rust (PyO3)", "rust"),
        ("Python (string)", "py_string"),
        ("Python (regex)", "py_regex"),
    ]:
        r = results.get(key)
        if r is None:
            print(f"{name:<20} {'N/A':>10} {'N/A':>12} {'N/A':>10}")
            continue
        us = r["ns_per_line"] / 1000
        lps = r["lines_per_sec"]
        if rust_lps and key != "rust":
            ratio = lps / rust_lps
            vs = f"{ratio:.2f}×"
        else:
            vs = "baseline"
        print(f"{name:<20} {us:>10.2f} {lps:>12,.0f} {vs:>10}")

    # Save JSON
    from pathlib import Path

    bench_dir = Path(__file__).parent.parent / ".benchmarks"
    bench_dir.mkdir(exist_ok=True)
    out = bench_dir / "py-parse-baseline.json"
    out.write_text(json.dumps(results, indent=2))
    print(f"\nResults saved to {out}")
