#!/usr/bin/env python3
"""Unified benchmark runner for trace_parser.

Runs all benchmark families and produces:
- .benchmarks/results/{commit}.json
- .benchmarks/results/latest.json (symlink)
- .benchmarks/report.md
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BENCH_DIR = ROOT / ".benchmarks" / "results"
BENCH_DIR.mkdir(parents=True, exist_ok=True)


def log(msg: str):
    ts = datetime.now().strftime("%H:%M:%S")
    print(f"[{ts}] {msg}", flush=True)


def run_cargo_bench(name_filter: str) -> str:
    """Run cargo bench with a name filter, return stdout."""
    log(f"Running cargo bench --bench {name_filter}...")
    start = time.perf_counter()
    result = subprocess.run(
        ["cargo", "bench", "--bench", name_filter, "--", "--noplot"],
        capture_output=True,
        text=True,
        cwd=ROOT,
    )
    elapsed = time.perf_counter() - start
    log(f"Cargo bench completed in {elapsed:.1f}s")
    return result.stdout


def fmt_speed(val: float | None) -> str:
    """Format lines/sec in a human-readable way."""
    if val is None:
        return "N/A"
    if val >= 1_000_000:
        return f"{val / 1_000_000:,.1f}M l/s"
    if val >= 1000:
        return f"{val / 1000:,.0f}K l/s"
    return f"{val:,.0f} l/s"


def compare_results(c1: str, c2: str):
    """Compare two benchmark results and print diff."""
    r1 = json.loads((BENCH_DIR / f"{c1}.json").read_text())
    r2 = json.loads((BENCH_DIR / f"{c2}.json").read_text())

    print(f"\n{'=' * 78}")
    print(f"BENCHMARK COMPARISON: {c1} → {c2}")
    print(f"{'=' * 78}\n")
    print(f"{'Metric':<35} {'Before':>12} {'After':>12} {'Change':>10} {'Status':>8}")
    print("-" * 78)

    b1 = next((b for b in r1["benchmarks"] if b["name"] == "rust_trace_parse"), None)
    b2 = next((b for b in r2["benchmarks"] if b["name"] == "rust_trace_parse"), None)
    if b1 and b2:
        change = (b2["lines_per_sec"] - b1["lines_per_sec"]) / b1["lines_per_sec"] * 100
        status = "✅" if change > 5 else "❌" if change < -5 else "➖"
        print(
            f"{'core/rust_trace_parse':<35} {fmt_speed(b1['lines_per_sec']):>12} {fmt_speed(b2['lines_per_sec']):>12} {change:>+9.1f}% {status:>8}"
        )
        print(
            f"  µs/line:{'':<26} {b1['ns_per_line'] / 1000:>11.2f} {b2['ns_per_line'] / 1000:>11.2f}"
        )
        print()

    # Event partial access comparison
    for section, suffix in [
        ("Event parse_only", "parse_only"),
        ("Event access_1", "access_1"),
        ("Event access_2", "access_2"),
        ("Event access_all", "access_all"),
    ]:
        print(
            f"{section:<35} {'Before':>12} {'After':>12} {'Change':>10} {'Status':>8}"
        )
        print("-" * 78)
        for name in [
            "TraceSchedSwitch",
            "TraceSchedWakeup",
            "TraceDevFrequency",
            "TraceMarkBegin",
            "TraceReceiveVsync",
        ]:
            b1 = next(
                (b for b in r1["benchmarks"] if b["name"] == f"{name}/{suffix}"), None
            )
            b2 = next(
                (b for b in r2["benchmarks"] if b["name"] == f"{name}/{suffix}"), None
            )
            if b1 and b2:
                change = (
                    (b2["lines_per_sec"] - b1["lines_per_sec"])
                    / b1["lines_per_sec"]
                    * 100
                )
                short = name
                status = "✅" if change > 5 else "❌" if change < -5 else "➖"
                print(
                    f"{short:<35} {fmt_speed(b1['lines_per_sec']):>12} {fmt_speed(b2['lines_per_sec']):>12} {change:>+9.1f}% {status:>8}"
                )
        print()

    # AOSP
    aosp_files = [
        "systrace_tutorial",
        "trace_30293222",
        "trace_30898724",
        "trace_30905547",
    ]
    has_aosp1 = any(b["family"] == "aosp" for b in r1["benchmarks"])
    has_aosp2 = any(b["family"] == "aosp" for b in r2["benchmarks"])
    if has_aosp1 and has_aosp2:
        print(
            f"{'AOSP File API — higher is better':<35} {'Before':>12} {'After':>12} {'Change':>10} {'Status':>8}"
        )
        print("-" * 78)
        for name in aosp_files:
            b1 = next((b for b in r1["benchmarks"] if b["name"] == name), None)
            b2 = next((b for b in r2["benchmarks"] if b["name"] == name), None)
            if b1 and b2:
                change = (
                    (b2["lines_per_sec"] - b1["lines_per_sec"])
                    / b1["lines_per_sec"]
                    * 100
                )
                status = "✅" if change > 5 else "❌" if change < -5 else "➖"
                print(
                    f"{name:<35} {fmt_speed(b1['lines_per_sec']):>12} {fmt_speed(b2['lines_per_sec']):>12} {change:>+9.1f}% {status:>8}"
                )

    print(f"\n{'=' * 78}")
    print("Legend: ✅ improved (>5%)  ❌ regressed (<-5%)  ➖ stable")
    print(f"{'=' * 78}\n")


def run_python_benchmarks() -> list[dict]:
    """Run Python-side benchmarks and return results."""
    log("Running Python benchmarks...")
    start = time.perf_counter()
    results = []

    from trace_parser import (
        TraceDevFrequency,
        TraceSchedSwitch,
        TraceSchedWakeup,
        parse_trace_file,
    )

    # AOSP File API benchmarks
    trace_dir = ROOT / "datasets" / "aosp" / "ftrace"
    trace_files = sorted(trace_dir.glob("*.trace")) if trace_dir.is_dir() else []

    for tf in trace_files:
        log(f"  Benchmarking {tf.name}...")
        file_start = time.perf_counter()
        lines_text = tf.read_text()
        lines = lines_text.splitlines()
        total_bytes = tf.stat().st_size
        total_lines = len(lines)

        events = parse_trace_file(str(tf))
        file_elapsed = time.perf_counter() - file_start

        results.append(
            {
                "family": "aosp",
                "name": tf.stem,
                "total_lines": total_lines,
                "total_bytes": total_bytes,
                "elapsed_sec": round(file_elapsed, 3),
                "ns_per_line": round(file_elapsed / total_lines * 1e9, 1)
                if total_lines
                else 0,
                "lines_per_sec": round(total_lines / file_elapsed, 1)
                if file_elapsed
                else 0,
                "bytes_per_sec": round(total_bytes / file_elapsed, 1)
                if file_elapsed
                else 0,
                "parse_rate": round(len(events) / total_lines * 100, 2)
                if total_lines
                else 0,
                "p50_ns": None,
                "p95_ns": None,
            }
        )

    # Python per-event partial access benchmarks
    log("  Python partial access benchmarks...")
    test_cases = [
        (
            "TraceSchedSwitch",
            TraceSchedSwitch,
            "bash-1977 (12) [000] .... 12345.678901: sched_switch: "
            "prev_comm=bash prev_pid=1977 prev_prio=120 prev_state=S ==> "
            "next_comm=worker next_pid=123 next_prio=120",
        ),
        (
            "TraceSchedWakeup",
            TraceSchedWakeup,
            "<idle>-0 (-----) [001] dn.4 2318.331005: sched_wakeup: "
            "comm=ksoftirqd/1 pid=12 prio=120 success=1 target_cpu=001",
        ),
        (
            "TraceDevFrequency",
            TraceDevFrequency,
            "swapper-0 (0) [000] .... 12345.678900: clock_set_rate: "
            "clk=ddr_devfreq state=933000000 cpu_id=0",
        ),
    ]

    for event_name, cls, line in test_cases:
        n = 10000
        line_bytes = len(line)

        # access_1: parse + 1 field
        t0 = time.perf_counter()
        for _ in range(n):
            e = cls.parse(line)
            if e:
                if hasattr(e, "prev_comm"):
                    _ = e.prev_comm
                elif hasattr(e, "comm"):
                    _ = e.comm
                elif hasattr(e, "state"):
                    _ = e.state
        access_1_elapsed = time.perf_counter() - t0

        # access_2: parse + 2 fields
        t0 = time.perf_counter()
        for _ in range(n):
            e = cls.parse(line)
            if e:
                if hasattr(e, "prev_comm"):
                    _ = e.prev_comm
                    _ = e.next_comm
                elif hasattr(e, "comm"):
                    _ = e.comm
                    _ = e.pid
                elif hasattr(e, "state"):
                    _ = e.state
                    _ = e.cpu_id
        access_2_elapsed = time.perf_counter() - t0

        # access_all: parse + all fields
        payload_attrs = []
        base_attrs = {
            "thread_name",
            "thread_tid",
            "thread_tgid",
            "cpu",
            "flags",
            "timestamp",
            "event_name",
            "payload_raw",
            "format_id",
            "cache",
            "dirty",
        }
        skip_attrs = {
            "parse",
            "can_be_parsed",
            "to_string",
            "payload",
            "template",
            "timestamp_ms",
            "timestamp_ns",
            "has_unknown_thread",
            "__class__",
            "__delattr__",
            "__dict__",
            "__dir__",
            "__doc__",
            "__eq__",
            "__format__",
            "__ge__",
            "__getattribute__",
            "__gt__",
            "__hash__",
            "__init__",
            "__init_subclass__",
            "__le__",
            "__lt__",
            "__module__",
            "__ne__",
            "__new__",
            "__reduce__",
            "__reduce_ex__",
            "__repr__",
            "__setattr__",
            "__sizeof__",
            "__str__",
            "__subclasshook__",
            "__copy__",
            "__deepcopy__",
            "__getstate__",
            "__setstate__",
        }
        for attr in dir(cls.parse(line) if cls.parse(line) else None):
            if (
                attr not in base_attrs
                and not attr.startswith("_")
                and attr not in skip_attrs
            ):
                if not callable(getattr(cls.parse(line), attr, None)):
                    payload_attrs.append(attr)

        t0 = time.perf_counter()
        for _ in range(n):
            e = cls.parse(line)
            if e:
                for attr in payload_attrs:
                    _ = getattr(e, attr, None)
        access_all_elapsed = time.perf_counter() - t0

        for suffix, elapsed in [
            ("access_1", access_1_elapsed),
            ("access_2", access_2_elapsed),
            ("access_all", access_all_elapsed),
        ]:
            results.append(
                {
                    "family": "python_access",
                    "name": f"{event_name}/{suffix}",
                    "total_lines": n,
                    "total_bytes": n * line_bytes,
                    "elapsed_sec": round(elapsed, 3),
                    "ns_per_line": round(elapsed / n * 1e9, 1) if n else 0,
                    "lines_per_sec": round(n / elapsed, 1) if elapsed else 0,
                    "bytes_per_sec": round(n * line_bytes / elapsed, 1)
                    if elapsed
                    else 0,
                    "parse_rate": None,
                    "p50_ns": None,
                    "p95_ns": None,
                }
            )

    elapsed = time.perf_counter() - start
    log(f"Python benchmarks completed in {elapsed:.1f}s")
    return results


def get_commit_info() -> str:
    """Get current commit hash."""
    try:
        return subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, text=True
        ).strip()
    except Exception:
        return "unknown"


def print_report(cargo_results: list[dict], python_results: list[dict]):
    """Print human-readable summary."""
    print("\n" + "=" * 70)
    print("BENCHMARK RESULTS")
    print("=" * 70)

    if cargo_results:
        print("\n--- Core Benchmarks ---")
        print(f"{'Name':<30} {'µs/line':>10} {'lines/sec':>12}")
        print("-" * 54)
        for r in cargo_results:
            if r["family"] == "core":
                us = r["ns_per_line"] / 1000
                print(f"{r['name']:<30} {us:>10,.2f} {r['lines_per_sec']:>12,.0f}")

        print("\n--- Event Benchmarks ---")
        print(f"{'Name':<40} {'µs/line':>10} {'lines/sec':>12}")
        print("-" * 64)
        for r in cargo_results:
            if r["family"] == "event":
                us = r["ns_per_line"] / 1000
                print(f"{r['name']:<40} {us:>10,.2f} {r['lines_per_sec']:>12,.0f}")

    if python_results:
        aosp = [r for r in python_results if r["family"] == "aosp"]
        if aosp:
            print("\n--- AOSP File API ---")
            print(f"{'File':<30} {'Lines':>8} {'lines/sec':>10} {'MiB/s':>8}")
            print("-" * 58)
            for r in aosp:
                mib = r["bytes_per_sec"] / 1_048_576 if r["bytes_per_sec"] else 0
                print(
                    f"{r['name']:<30} {r['total_lines']:>8,} {r['lines_per_sec']:>10,.0f} {mib:>8.1f}"
                )

        py_access = [r for r in python_results if r["family"] == "python_access"]
        if py_access:
            print("\n--- Python Partial Access ---")
            print(f"{'Name':<40} {'µs/line':>10} {'lines/sec':>12}")
            print("-" * 64)
            for r in py_access:
                us = r["ns_per_line"] / 1000
                print(f"{r['name']:<40} {us:>10,.2f} {r['lines_per_sec']:>12,.0f}")

    print("\n" + "=" * 70)


def main():
    parser = argparse.ArgumentParser(
        description="Unified benchmark runner for trace_parser"
    )
    parser.add_argument(
        "--compare",
        nargs=2,
        metavar=("COMMIT1", "COMMIT2"),
        help="Compare results between two commits",
    )
    parser.add_argument(
        "--list", action="store_true", help="List available benchmark results"
    )
    args = parser.parse_args()

    if args.compare:
        compare_results(args.compare[0], args.compare[1])
        return

    if args.list:
        print("Available benchmark results:")
        for f in sorted(BENCH_DIR.glob("*.json")):
            if f.name == "latest.json":
                continue
            print(f"  {f.stem}")
        return

    overall_start = time.perf_counter()

    # Check Python module first (fail fast)
    log("Checking trace_parser import...")
    try:
        from trace_parser import parse_trace_file  # noqa: F401
    except ImportError:
        log("ERROR: trace_parser module not found. Run 'maturin develop' first.")
        sys.exit(1)
    log("trace_parser module available ✓")

    log("Phase 1: Rust benchmarks (cargo bench)")
    cargo_output = run_cargo_bench("throughput")
    cargo_results = parse_criterion_results(cargo_output)

    log("Phase 2: Python benchmarks")
    python_results = run_python_benchmarks()

    # Aggregate
    commit = get_commit_info()
    report = {
        "commit": commit,
        "date": datetime.now(timezone.utc).isoformat(),
        "host": f"{os.uname().sysname.lower()}-{os.uname().machine}",
        "benchmarks": cargo_results + python_results,
    }

    # Save
    out_path = BENCH_DIR / f"{commit}.json"
    out_path.write_text(json.dumps(report, indent=2))
    latest = BENCH_DIR / "latest.json"
    if latest.exists() or latest.is_symlink():
        latest.unlink()
    latest.symlink_to(out_path.name)

    elapsed = time.perf_counter() - overall_start
    log(f"Total benchmark time: {elapsed:.1f}s")
    log(f"Results saved to {out_path}")

    # Print summary
    print_report(cargo_results, python_results)


def parse_criterion_results(text: str) -> list[dict]:
    """Parse all benchmark results from cargo bench output."""
    results = []
    bench_pattern = re.compile(
        r"^(core/\S+|event/\S+)\s+time:\s+\[\s*[\d.]+\s*(\w+)\s+[\d.]+\s*(\w+)\s+([\d.]+)\s*(\w+)\s*\]",
        re.MULTILINE,
    )

    for m in bench_pattern.finditer(text):
        name = m.group(1)
        val = float(m.group(4))
        unit = m.group(5)
        if unit == "ns":
            ns_total = val
        elif unit == "µs":
            ns_total = val * 1_000
        elif unit == "ms":
            ns_total = val * 1_000_000
        elif unit == "s":
            ns_total = val * 1_000_000_000
        else:
            continue

        family = name.split("/")[0]
        bench_name = "/".join(name.split("/")[1:])
        batch = BATCH_SIZES.get(name, 1)
        ns_per_line = ns_total / batch

        results.append(
            {
                "family": family,
                "name": bench_name,
                "ns_per_line": round(ns_per_line, 1),
                "lines_per_sec": round(1e9 / ns_per_line, 1) if ns_per_line > 0 else 0,
            }
        )

    return results


# Known batch sizes (must match benches/throughput.rs)
BATCH_SIZES = {
    "core/rust_trace_parse": 5000,
    "event/TraceSchedSwitch/parse_only": 200,
    "event/TraceSchedSwitch/access_1": 200,
    "event/TraceSchedSwitch/access_2": 200,
    "event/TraceSchedSwitch/access_all": 200,
    "event/TraceSchedSwitch/negative": 200,
    "event/TraceSchedWakeup/parse_only": 200,
    "event/TraceSchedWakeup/access_1": 200,
    "event/TraceSchedWakeup/access_2": 200,
    "event/TraceSchedWakeup/access_all": 200,
    "event/TraceSchedWakeup/negative": 200,
    "event/TraceDevFrequency/parse_only": 200,
    "event/TraceDevFrequency/access_1": 200,
    "event/TraceDevFrequency/access_2": 200,
    "event/TraceDevFrequency/access_all": 200,
    "event/TraceDevFrequency/negative": 200,
    "event/TraceMarkBegin/parse_only": 200,
    "event/TraceMarkBegin/access_1": 200,
    "event/TraceMarkBegin/access_2": 200,
    "event/TraceMarkBegin/access_all": 200,
    "event/TraceMarkBegin/negative": 200,
    "event/TraceReceiveVsync/parse_only": 200,
    "event/TraceReceiveVsync/access_1": 200,
    "event/TraceReceiveVsync/access_2": 200,
    "event/TraceReceiveVsync/access_all": 200,
    "event/TraceReceiveVsync/negative": 200,
}


if __name__ == "__main__":
    main()
