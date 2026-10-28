#!/usr/bin/env python3
"""E0 host-oracle *simulator*; not actual QEMU/QMP integration.

QEMU isa-debug-exit write byte 0x20 -> process status (0x20 << 1) | 1 = 65.
Host reset, timeout, incorrect case, missing or premature markers must fail.
"""
from dataclasses import dataclass
from typing import Sequence

EXPECTED_EXIT = (0x20 << 1) | 1
STAGES = (
    "UNIOS:L1:EXITBS_OK",
    "UNIOS:L1:CR3_OK",
    "UNIOS:L1:KERNEL_ENTRY",
    "UNIOS:L1:PROTECTIONS_OK",
    "UNIOS:L1:HALTED_PASS",
)

@dataclass(frozen=True)
class Observation:
    case_id: str
    expected_case_id: str
    serial: str
    exit_code: int | None
    timed_out: bool
    qmp_events: Sequence[str]

def clean_pass(run: Observation) -> bool:
    if run.case_id != run.expected_case_id or run.case_id != "L1-X01":
        return False
    if run.exit_code != EXPECTED_EXIT or run.timed_out:
        return False
    if any(event in ("RESET", "SHUTDOWN", "GUEST_PANICKED") for event in run.qmp_events):
        return False
    lines = run.serial.splitlines()
    if lines.count(STAGES[-1]) != 1:
        return False
    positions = [next((i for i, line in enumerate(lines) if line == marker), -1)
                 for marker in STAGES]
    return all(i >= 0 for i in positions) and positions == sorted(set(positions))
