#!/usr/bin/env python3
"""Extract ftrace lines from AOSP systrace HTML files.

Usage:
  python3 benches/extract_aosp_ftrace.py \
      --input-glob '.benchmarks/aosp-traces/*.html' \
      --output-dir datasets/aosp/ftrace
"""

from __future__ import annotations

import argparse
import glob
import re
from pathlib import Path

LINE_RE = re.compile(r"^.+-\d+\s+\((\s*\d+|-----)\)\s+\[\d+\]\s+.*?:\s+[^:]+:\s+.+$")


def extract_html_to_trace(input_path: Path, out_path: Path) -> int:
    count = 0
    with (
        input_path.open("r", encoding="utf-8", errors="ignore") as src,
        out_path.open("w", encoding="utf-8") as dst,
    ):
        for line in src:
            line = line.rstrip("\n")
            if LINE_RE.match(line):
                dst.write(line + "\n")
                count += 1
    return count


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--input-glob", required=True)
    p.add_argument("--output-dir", required=True)
    args = p.parse_args()

    files = sorted(glob.glob(args.input_glob))
    if not files:
        print(f"No inputs matched: {args.input_glob}")
        return 1

    out_dir = Path(args.output_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    print("input\tlines\toutput")
    for file in files:
        input_path = Path(file)
        out_path = out_dir / f"{input_path.stem}.trace"
        lines = extract_html_to_trace(input_path, out_path)
        print(f"{input_path}\t{lines}\t{out_path}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
