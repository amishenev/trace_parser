#!/usr/bin/env python3
"""Offline benchmark runner for extracted AOSP trace files.

Default input glob:
  datasets/aosp/ftrace/*.trace

Examples:
  uv run python benches/aosp_benchmark.py
  uv run python benches/aosp_benchmark.py --mode both --max-files 2 --max-lines 200000
  uv run python benches/aosp_benchmark.py --mode file --json-out .benchmarks/aosp-bench/results.json --csv-out .benchmarks/aosp-bench/results.csv
"""

from __future__ import annotations

import argparse
import csv
import glob
import json
import time
from collections import Counter
from pathlib import Path
from typing import Any


def human_bps(value: float) -> str:
    units = ["B/s", "KiB/s", "MiB/s", "GiB/s"]
    idx = 0
    while value >= 1024.0 and idx < len(units) - 1:
        value /= 1024.0
        idx += 1
    return f"{value:.2f} {units[idx]}"


def benchmark_line_mode(path: str, max_lines: int | None) -> dict[str, Any]:
    from trace_parser import parse_trace

    total = 0
    parsed = 0
    total_bytes = 0
    by_type: Counter[str] = Counter()

    start = time.perf_counter()
    with open(path, "r", encoding="utf-8", errors="ignore") as f:
        for line in f:
            if max_lines is not None and total >= max_lines:
                break
            total += 1
            total_bytes += len(line.encode("utf-8", errors="ignore"))
            event = parse_trace(line.rstrip("\n"))
            if event is not None:
                parsed += 1
                by_type[type(event).__name__] += 1
    elapsed = time.perf_counter() - start

    return {
        "mode": "line",
        "file": path,
        "total_lines": total,
        "parsed_lines": parsed,
        "parse_rate": (parsed / total) if total else 0.0,
        "elapsed_sec": elapsed,
        "lines_per_sec": (total / elapsed) if elapsed else 0.0,
        "bytes_per_sec": (total_bytes / elapsed) if elapsed else 0.0,
        "top_types": by_type.most_common(10),
    }


def benchmark_file_mode(path: str) -> dict[str, Any]:
    from trace_parser import parse_trace_file

    total = 0
    total_bytes = 0
    with open(path, "r", encoding="utf-8", errors="ignore") as f:
        for line in f:
            total += 1
            total_bytes += len(line.encode("utf-8", errors="ignore"))

    start = time.perf_counter()
    events = parse_trace_file(path)
    elapsed = time.perf_counter() - start
    by_type: Counter[str] = Counter(type(e).__name__ for e in events)

    parsed = len(events)
    return {
        "mode": "file",
        "file": path,
        "total_lines": total,
        "parsed_lines": parsed,
        "parse_rate": (parsed / total) if total else 0.0,
        "elapsed_sec": elapsed,
        "lines_per_sec": (total / elapsed) if elapsed else 0.0,
        "bytes_per_sec": (total_bytes / elapsed) if elapsed else 0.0,
        "top_types": by_type.most_common(10),
    }


def write_json(path: str | None, rows: list[dict[str, Any]]) -> None:
    if not path:
        return
    out = Path(path)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(rows, ensure_ascii=False, indent=2), encoding="utf-8")


def write_csv(path: str | None, rows: list[dict[str, Any]]) -> None:
    if not path:
        return
    out = Path(path)
    out.parent.mkdir(parents=True, exist_ok=True)
    fields = [
        "mode",
        "file",
        "total_lines",
        "parsed_lines",
        "parse_rate",
        "elapsed_sec",
        "lines_per_sec",
        "bytes_per_sec",
    ]
    with out.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            writer.writerow({k: row[k] for k in fields})


def print_table(rows: list[dict[str, Any]]) -> None:
    print("mode\tlines\tparsed\trate\tsec\tlines_per_sec\tthroughput\tfile")
    for r in rows:
        print(
            "\t".join(
                [
                    str(r["mode"]),
                    str(r["total_lines"]),
                    str(r["parsed_lines"]),
                    f"{float(r['parse_rate']):.2%}",
                    f"{float(r['elapsed_sec']):.3f}",
                    f"{float(r['lines_per_sec']):.1f}",
                    human_bps(float(r["bytes_per_sec"])),
                    str(r["file"]),
                ]
            )
        )


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--input-glob",
        default="datasets/aosp/ftrace/*.trace",
        help="Input trace file glob",
    )
    p.add_argument(
        "--mode",
        choices=["line", "file", "both"],
        default="both",
        help="Benchmark mode",
    )
    p.add_argument("--max-files", type=int, default=None, help="Limit files")
    p.add_argument(
        "--max-lines",
        type=int,
        default=None,
        help="Limit lines per file for line mode only",
    )
    p.add_argument(
        "--json-out",
        default=".benchmarks/aosp-bench/results.json",
        help="Output JSON path (empty to disable)",
    )
    p.add_argument(
        "--csv-out",
        default=".benchmarks/aosp-bench/results.csv",
        help="Output CSV path (empty to disable)",
    )
    return p


def main() -> int:
    args = build_parser().parse_args()
    files = sorted(glob.glob(args.input_glob))
    if args.max_files is not None:
        files = files[: args.max_files]

    if not files:
        print(f"No files matched: {args.input_glob}")
        return 1

    rows: list[dict[str, Any]] = []
    for path in files:
        print(f"[bench] {path}")
        if args.mode in {"line", "both"}:
            rows.append(benchmark_line_mode(path, args.max_lines))
        if args.mode in {"file", "both"}:
            rows.append(benchmark_file_mode(path))

    print()
    print_table(rows)

    json_out = args.json_out.strip() if args.json_out is not None else None
    csv_out = args.csv_out.strip() if args.csv_out is not None else None

    write_json(json_out or None, rows)
    write_csv(csv_out or None, rows)

    if json_out:
        print(f"\nSaved JSON: {json_out}")
    if csv_out:
        print(f"Saved CSV: {csv_out}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
