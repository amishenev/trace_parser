# AOSP Offline Traces

This directory contains offline `ftrace` text traces extracted from AOSP systrace tutorial HTML samples.

## Files

- `ftrace/systrace_tutorial.trace`
- `ftrace/trace_30293222.trace`
- `ftrace/trace_30898724.trace`
- `ftrace/trace_30905547.trace`

## Source

AOSP docs repository blob (commit-pinned):

- `https://android.googlesource.com/platform/docs/source.android.com/+/1d10de1462c027a2ccfb429e85b1172db6d58e53/en/devices/tech/debug/perf_traces.zip?format=TEXT`

## Regeneration

```bash
python3 benches/extract_aosp_ftrace.py \
  --input-glob '.benchmarks/aosp-traces/*.html' \
  --output-dir datasets/aosp/ftrace
```

Extraction rule: keep only lines matching the trace shape
`TASK-TID (TGID) [CPU] FLAGS TIMESTAMP: event_name: payload`.
