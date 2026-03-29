#!/usr/bin/env python3
"""Generate a test trace file with mixed events for benchmarks."""

import random
import sys


def generate_line(event: str, i: int) -> str:
    """Generate a realistic trace line for the given event type."""
    timestamp = 12345.678900 + i * 0.000001
    cpu = i % 4
    tid = 1000 + i % 100
    tgid = 1000 + i % 50

    if event == "sched_switch":
        return (
            f"bash-{tid} ({tgid}) [{cpu:03d}] .... {timestamp:.6f}: sched_switch: "
            f"prev_comm=bash prev_pid={tid} prev_prio=120 prev_state=S ==> "
            f"next_comm=worker next_pid={tid + 1} next_prio=120"
        )
    elif event == "sched_wakeup":
        return (
            f"kworker-{tid} ({tgid}) [{cpu:03d}] .... {timestamp:.6f}: sched_wakeup: "
            f"comm=bash pid={tid} prio=120 target_cpu={cpu:03d}"
        )
    elif event == "sched_process_exit":
        return (
            f"bash-{tid} ({tgid}) [{cpu:03d}] .... {timestamp:.6f}: sched_process_exit: "
            f"comm=bash pid={tid} prio=120 group_dead=0"
        )
    elif event == "clock_set_rate":
        clk = random.choice(["ddr_devfreq", "l3c_devfreq"])
        state = random.choice([600000000, 933000000, 1066000000])
        return (
            f"swapper-0 (0) [{cpu:03d}] .... {timestamp:.6f}: clock_set_rate: "
            f"clk={clk} state={state} cpu_id={cpu}"
        )
    else:
        raise ValueError(f"Unknown event: {event}")


def generate_trace_file(path: str, lines: int = 100000, seed: int = 42) -> None:
    """Generate a test trace file with mixed events."""
    random.seed(seed)

    events = [
        "sched_switch",
        "sched_wakeup",
        "sched_process_exit",
        "clock_set_rate",
    ]

    with open(path, "w") as f:
        for i in range(lines):
            event = random.choice(events)
            f.write(generate_line(event, i) + "\n")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <output_path> [num_lines]")
        sys.exit(1)

    output_path = sys.argv[1]
    num_lines = int(sys.argv[2]) if len(sys.argv) > 2 else 100000

    generate_trace_file(output_path, num_lines)
    print(f"Generated {num_lines} trace lines to {output_path}")
