#!/usr/bin/env python3
"""Host tests for the raw board-evidence validator."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import validate


def run_start() -> dict[str, object]:
    return {
        "record": "run_start",
        "schema": validate.EXPECTED_SCHEMA,
        "workload_id": validate.EXPECTED_WORKLOAD,
        "expected_cases": 256,
        "warmup_iterations": 1,
        "sample_count": 2,
        "cycle_limit": validate.EXPECTED_CYCLE_LIMIT,
        "stack_limit_bytes": validate.EXPECTED_STACK_LIMIT,
        "target": "thumbv7em-none-eabihf",
        "board": "STM32F407G-DISC1",
        "cpu": "STM32F407VGT6 Cortex-M4F r0p1",
        "cpuid": 0x410FC241,
        "device_idcode": 0x10000413,
        "clock_hz": 168_000_000,
        "clock_source": "8MHz-HSE-PLL",
        "flash_wait_states": 5,
        "flash_prefetch": True,
        "flash_instruction_cache": True,
        "flash_data_cache": True,
        "code_placement": "flash",
        "project_stack_placement": "sram",
        "main_stack_placement": "ccm",
        "allocator": "first-fit-coalescing-static-48k-sram-v1",
        "masked_run_interrupts": "PRIMASK=1",
        "application_irq_load": (
            "one TIM2 update IRQ per projection, 84MHz timer, ARR=32, "
            "ISR asserts cancellation and timestamps entry/exit"
        ),
        "rcc_cr": (1 << 16) | (1 << 17) | (1 << 24) | (1 << 25),
        "rcc_pllcfgr": 8 | (336 << 6) | (1 << 22) | (7 << 24),
        "rcc_cfgr": 0b10 | (0b10 << 2) | (0b101 << 10) | (0b100 << 13),
        "flash_acr": 5 | (1 << 8) | (1 << 9) | (1 << 10),
    }


def case(index: int) -> dict[str, object]:
    lexeme = "1" + "0" * index
    return {
        "record": "case",
        "case_index": index,
        "family": "validator-fixture",
        "lexeme": lexeme,
        "length": len(lexeme),
        "projection": {"kind": "finite", "bits": "0x3ff0000000000000"},
        "masked": {
            "cold_cycles": 9,
            "stack_pattern_b_cycles": 12,
            "max_cycles": 12,
            "allocation_calls": 0,
            "stack_pattern_a_depth_bytes": 100,
            "stack_pattern_b_depth_bytes": 104,
            "stack_watermark_conservative_depth_bytes": 104,
            "samples": [10, 11],
        },
        "irq_loaded": {
            "cold_cycles": 14,
            "stack_pattern_b_cycles": 17,
            "max_cycles": 17,
            "allocation_calls": 0,
            "cancellation_observed": 1,
            "cancellation_expected": 1,
            "stack_pattern_a_project_depth_bytes": 110,
            "stack_pattern_b_project_depth_bytes": 112,
            "project_stack_watermark_conservative_depth_bytes": 112,
            "stack_pattern_a_interrupt_depth_bytes": 32,
            "stack_pattern_b_interrupt_depth_bytes": 36,
            "interrupt_stack_watermark_conservative_depth_bytes": 36,
            "samples": [15, 16],
            "cancellation_assertion_delay_cycles": [2, 2],
            "cancellation_latency_cycles": [5, 6],
        },
        "result_mismatches": 0,
        "guard_ok": True,
        "stack_watermark_certain": True,
        "failed": False,
    }


class ValidatorTests(unittest.TestCase):
    def write_evidence(self, directory: Path, records: list[dict[str, object]]) -> tuple[Path, Path]:
        raw = directory / "raw.jsonl"
        raw.write_text(
            "".join(json.dumps(record, separators=(",", ":")) + "\n" for record in records),
            encoding="utf-8",
        )
        coverage = directory / "coverage.txt"
        coverage.write_text("core/src/num/dec2flt/slow.rs\n", encoding="utf-8")
        return raw, coverage

    @patch.object(validate, "EXPECTED_CASES", 256)
    @patch.object(validate, "EXPECTED_SAMPLES", 2)
    @patch.object(validate, "EXPECTED_WARMUPS", 1)
    def test_accepts_complete_consistent_candidate(self) -> None:
        records = [run_start(), *(case(index) for index in range(256))]
        records.append(
            {
                "record": "run_end",
                "workload_id": validate.EXPECTED_WORKLOAD,
                "case_count": 256,
                "failed_cases": 0,
                "complete": True,
            }
        )
        with tempfile.TemporaryDirectory() as temporary:
            raw, coverage = self.write_evidence(Path(temporary), records)
            summary, passed = validate.validate(raw, coverage)
        self.assertTrue(passed)
        self.assertEqual(summary["case_count"], 256)
        self.assertEqual(summary["status"], "candidate-passed")
        self.assertFalse(summary["admission_claim"])

    @patch.object(validate, "EXPECTED_CASES", 256)
    @patch.object(validate, "EXPECTED_SAMPLES", 2)
    @patch.object(validate, "EXPECTED_WARMUPS", 1)
    def test_rejects_inconsistent_raw_maximum(self) -> None:
        records = [run_start(), *(case(index) for index in range(256))]
        records[1]["masked"]["max_cycles"] = 11
        records.append(
            {
                "record": "run_end",
                "workload_id": validate.EXPECTED_WORKLOAD,
                "case_count": 256,
                "failed_cases": 0,
                "complete": True,
            }
        )
        with tempfile.TemporaryDirectory() as temporary:
            raw, coverage = self.write_evidence(Path(temporary), records)
            with self.assertRaisesRegex(validate.InvalidEvidence, "masked max mismatch"):
                validate.validate(raw, coverage)


if __name__ == "__main__":
    unittest.main()
