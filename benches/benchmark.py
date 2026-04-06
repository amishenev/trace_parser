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


# Known batch sizes (must match benches/throughput.rs)
BATCH_SIZES = {
    "core/rust_trace_parse": 5000,
    "event/TraceSchedSwitch/positive": 200,
    "event/TraceSchedSwitch/negative": 200,
    "event/TraceSchedWakeup/positive": 200,
    "event/TraceSchedWakeup/negative": 200,
    "event/TraceDevFrequency/positive": 200,
    "event/TraceDevFrequency/negative": 200,
    "event/TraceMarkBegin/positive": 200,
    "event/TraceMarkBegin/negative": 200,
    "event/TraceReceiveVsync/positive": 200,
    "event/TraceReceiveVsync/negative": 200,
}


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


def run_python_benchmarks() -> list[dict]:
    """Run Python-side benchmarks and return results."""
    log("Running Python benchmarks...")
    start = time.perf_counter()
    results = []

    from trace_parser import parse_trace_file

    trace_dir = ROOT / "datasets" / "aosp" / "ftrace"
    trace_files = sorted(trace_dir.glob("*.trace")) if trace_dir.is_dir() else []

    for tf in trace_files:
        log(f"  Benchmarking {tf.name}...")
        file_start = time.perf_counter()
        lines = tf.read_text().splitlines()
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

        print("\n--- Event Benchmarks (full parse) ---")
        print(f"{'Name':<40} {'µs/line':>10} {'lines/sec':>12}")
        print("-" * 64)
        for r in cargo_results:
            if r["family"] == "event" and "positive" in r["name"]:
                us = r["ns_per_line"] / 1000
                print(f"{r['name']:<40} {us:>10,.2f} {r['lines_per_sec']:>12,.0f}")

        print("\n--- Event Benchmarks (quick-check rejection) ---")
        print(f"{'Name':<40} {'µs/line':>10} {'lines/sec':>12}")
        print("-" * 64)
        for r in cargo_results:
            if r["family"] == "event" and "negative" in r["name"]:
                us = r["ns_per_line"] / 1000
                print(f"{r['name']:<40} {us:>10,.2f} {r['lines_per_sec']:>12,.0f}")

    if python_results:
        print("\n--- AOSP File API ---")
        print(f"{'File':<30} {'Lines':>8} {'lines/sec':>10} {'MiB/s':>8}")
        print("-" * 58)
        for r in python_results:
            mib = r["bytes_per_sec"] / 1_048_576 if r["bytes_per_sec"] else 0
            print(
                f"{r['name']:<30} {r['total_lines']:>8,} {r['lines_per_sec']:>10,.0f} {mib:>8.1f}"
            )

    print("\n" + "=" * 70)


def fmt_lines(val: float | None) -> str:
    if val is None:
        return "N/A"
    if val >= 1_000_000:
        return f"{val / 1_000_000:,.0f}M"
    if val >= 1000:
        return f"{val / 1000:,.0f}K"
    return f"{val:,.0f}"


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

    print(
        f"{'Event (full parse — higher is better)':<35} {'Before':>12} {'After':>12} {'Change':>10} {'Status':>8}"
    )
    print("-" * 78)
    for name in [
        "TraceSchedSwitch/positive",
        "TraceSchedWakeup/positive",
        "TraceDevFrequency/positive",
        "TraceMarkBegin/positive",
        "TraceReceiveVsync/positive",
    ]:
        b1 = next((b for b in r1["benchmarks"] if b["name"] == name), None)
        b2 = next((b for b in r2["benchmarks"] if b["name"] == name), None)
        if b1 and b2:
            change = (
                (b2["lines_per_sec"] - b1["lines_per_sec"]) / b1["lines_per_sec"] * 100
            )
            short = name.replace("/positive", "")
            status = "✅" if change > 5 else "❌" if change < -5 else "➖"
            print(
                f"{short:<35} {fmt_speed(b1['lines_per_sec']):>12} {fmt_speed(b2['lines_per_sec']):>12} {change:>+9.1f}% {status:>8}"
            )

    print(
        f"\n{'Quick-check rejection — higher is better':<35} {'Before':>12} {'After':>12} {'Change':>10} {'Status':>8}"
    )
    print("-" * 78)
    for name in [
        "TraceSchedSwitch/negative",
        "TraceSchedWakeup/negative",
        "TraceDevFrequency/negative",
        "TraceMarkBegin/negative",
        "TraceReceiveVsync/negative",
    ]:
        b1 = next((b for b in r1["benchmarks"] if b["name"] == name), None)
        b2 = next((b for b in r2["benchmarks"] if b["name"] == name), None)
        if b1 and b2:
            change = (
                (b2["lines_per_sec"] - b1["lines_per_sec"]) / b1["lines_per_sec"] * 100
            )
            short = name.replace("/negative", "")
            # Negative benchmarks: lower rejection speed means slower, but we want fast
            status = "✅" if change > 5 else "❌" if change < -5 else "➖"
            print(
                f"{short:<35} {fmt_speed(b1['lines_per_sec']):>12} {fmt_speed(b2['lines_per_sec']):>12} {change:>+9.1f}% {status:>8}"
            )

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
            f"\n{'AOSP File API — higher is better':<35} {'Before':>12} {'After':>12} {'Change':>10} {'Status':>8}"
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


if __name__ == "__main__":
    main()
