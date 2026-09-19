#!/usr/bin/env python3
"""Host tests for the raw board-evidence validator."""

from __future__ import annotations

import hashlib
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
            "cold_cancellation_assertion_delay_cycles": 14,
            "cold_cancellation_latency_cycles": 7,
            "stack_pattern_b_cycles": 17,
            "stack_pattern_b_cancellation_assertion_delay_cycles": 16,
            "stack_pattern_b_cancellation_latency_cycles": 8,
            "max_cycles": 17,
            "allocation_calls": 0,
            "cancellation_observed": 5,
            "cancellation_expected": 5,
            "stack_pattern_a_project_depth_bytes": 110,
            "stack_pattern_b_project_depth_bytes": 112,
            "project_stack_watermark_conservative_depth_bytes": 112,
            "stack_pattern_a_interrupt_depth_bytes": 32,
            "stack_pattern_b_interrupt_depth_bytes": 36,
            "interrupt_stack_watermark_conservative_depth_bytes": 36,
            "samples": [15, 16],
            "cancellation_assertion_delay_cycles": [12, 12],
            "cancellation_latency_cycles": [5, 6],
        },
        "result_mismatches": 0,
        "guard_ok": True,
        "stack_watermark_certain": True,
        "failed": False,
    }


@patch.object(validate, "EXPECTED_CASES", 256)
@patch.object(validate, "EXPECTED_SAMPLES", 2)
@patch.object(validate, "EXPECTED_WARMUPS", 1)
class ValidatorTests(unittest.TestCase):
    def coverage_record(self, firmware: Path) -> dict[str, object]:
        firmware_sha256 = hashlib.sha256(firmware.read_bytes()).hexdigest()
        return {
            "schema": validate.COVERAGE_SCHEMA,
            "firmware_sha256": firmware_sha256,
            "breakpoint": {
                "number": 1,
                "kind": "hardware",
                "location": validate.COVERAGE_LOCATION,
                "resolved_addresses": ["0x080144b0"],
            },
            "stop": {
                "reason": "breakpoint-hit",
                "breakpoint_number": 1,
                "pc": "0x080144b0",
            },
        }

    def write_evidence(
        self,
        directory: Path,
        records: list[dict[str, object]],
        coverage_text: str | None = None,
    ) -> tuple[Path, Path, Path]:
        raw = directory / "raw.jsonl"
        raw.write_text(
            "".join(json.dumps(record, separators=(",", ":")) + "\n" for record in records),
            encoding="utf-8",
        )
        firmware = directory / "firmware.elf"
        firmware.write_bytes(b"exact prepared ELF fixture")
        coverage = directory / "coverage.txt"
        coverage.write_text(
            coverage_text
            or json.dumps(self.coverage_record(firmware), separators=(",", ":")) + "\n",
            encoding="utf-8",
        )
        return raw, coverage, firmware

    def validate_case(
        self, first_case: dict[str, object], coverage_text: str | None = None
    ) -> tuple[dict[str, object], bool]:
        records = [run_start(), *(case(index) for index in range(256))]
        records[1] = first_case
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
            raw, coverage, firmware = self.write_evidence(
                Path(temporary), records, coverage_text
            )
            return validate.validate(raw, coverage, firmware)

    def test_accepts_complete_consistent_candidate(self) -> None:
        summary, passed = self.validate_case(case(0))
        self.assertTrue(passed)
        self.assertEqual(summary["case_count"], 256)
        self.assertEqual(summary["status"], "candidate-passed")
        self.assertFalse(summary["admission_claim"])
        self.assertEqual(summary["cancellation_latency_max_cycles"], 8)
        self.assertEqual(summary["cancellation_assertion_delay_max_cycles"], 16)

    def test_rejects_echoed_gdb_command_arguments(self) -> None:
        echoed = (
            'Spawning Command { std: "gdb", args: ["-ex", '
            '"printf \\"WP100_SLOW_FALLBACK_HIT\\\\n\\"", '
            '"hbreak library/core/src/num/dec2flt/slow.rs:39"] }\n'
        )
        with self.assertRaisesRegex(validate.InvalidEvidence, "valid JSON"):
            self.validate_case(case(0), echoed)

    def test_rejects_unrelated_stop_even_with_old_marker_and_location(self) -> None:
        unrelated = (
            "Program received signal SIGTRAP, Trace/breakpoint trap.\n"
            "WP100_SLOW_FALLBACK_HIT\n"
            "library/core/src/num/dec2flt/slow.rs:39\n"
        )
        with self.assertRaisesRegex(validate.InvalidEvidence, "machine-readable"):
            self.validate_case(case(0), unrelated)

    def test_rejects_structured_record_for_a_different_breakpoint(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            firmware = Path(temporary) / "firmware.elf"
            firmware.write_bytes(b"exact prepared ELF fixture")
            record = self.coverage_record(firmware)
            record["stop"]["breakpoint_number"] = 2
            coverage_text = json.dumps(record, separators=(",", ":")) + "\n"
        with self.assertRaisesRegex(validate.InvalidEvidence, "different breakpoint"):
            self.validate_case(case(0), coverage_text)

    def test_rejects_breakpoint_location_without_runtime_hit_record(self) -> None:
        with self.assertRaisesRegex(validate.InvalidEvidence, "valid JSON"):
            self.validate_case(case(0), "core/src/num/dec2flt/slow.rs\n")

    def test_rejects_inconsistent_raw_maximum(self) -> None:
        record = case(0)
        record["masked"]["max_cycles"] = 11
        with self.assertRaisesRegex(validate.InvalidEvidence, "masked max mismatch"):
            self.validate_case(record)

    def set_irq_interval(
        self, record: dict[str, object], form: str, start: int, stop: int, irq: int
    ) -> None:
        loaded = record["irq_loaded"]
        cycles = (stop - start) & 0xFFFFFFFF
        latency = (stop - irq) & 0xFFFFFFFF
        delay = (irq - (start - 10)) & 0xFFFFFFFF
        if form == "sample":
            loaded["samples"][0] = cycles
            loaded["cancellation_assertion_delay_cycles"][0] = delay
            # Firmware serializes its u32::MAX sentinel as null in arrays.
            loaded["cancellation_latency_cycles"][0] = None if latency == 0xFFFFFFFF else latency
        else:
            loaded[f"{form}_cycles"] = cycles
            loaded[f"{form}_cancellation_assertion_delay_cycles"] = delay
            loaded[f"{form}_cancellation_latency_cycles"] = latency
        loaded["max_cycles"] = max(
            loaded["cold_cycles"], loaded["stack_pattern_b_cycles"], *loaded["samples"]
        )

    def test_rejects_irq_outside_each_measured_interval(self) -> None:
        for form in ("sample", "cold", "stack_pattern_b"):
            # Before start can have a plausible latency smaller than case max;
            # after stop wraps even though cancellation_observed is complete.
            for start in (100, 0xFFFFFFF8):
                stop = (start + 15) & 0xFFFFFFFF
                outside_irqs = (start - 1, stop + 1, stop + 2)
                for irq in (value & 0xFFFFFFFF for value in outside_irqs):
                    with self.subTest(form=form, start=start, irq=irq):
                        record = case(0)
                        self.set_irq_interval(record, form, start, stop, irq)
                        with self.assertRaises(validate.InvalidEvidence):
                            self.validate_case(record)

    def test_accepts_irq_inside_each_interval_including_counter_wrap(self) -> None:
        for form in ("sample", "cold", "stack_pattern_b"):
            for start in (100, 0xFFFFFFF8):
                stop = (start + 15) & 0xFFFFFFFF
                for offset in (0, 7, 15):
                    with self.subTest(form=form, start=start, offset=offset):
                        record = case(0)
                        self.set_irq_interval(record, form, start, stop, (start + offset) & 0xFFFFFFFF)
                        _, passed = self.validate_case(record)
                        self.assertTrue(passed)

    def test_requires_valid_cold_and_stack_b_cancellation_fields(self) -> None:
        for form in ("cold", "stack_pattern_b"):
            for suffix in ("cancellation_assertion_delay_cycles", "cancellation_latency_cycles"):
                field = f"{form}_{suffix}"
                for invalid in (None, True, -1, 1.5, "7", 0xFFFFFFFF, 0x100000000, "missing"):
                    with self.subTest(field=field, invalid=invalid):
                        record = case(0)
                        if invalid == "missing":
                            del record["irq_loaded"][field]
                        else:
                            record["irq_loaded"][field] = invalid
                        with self.assertRaises(validate.InvalidEvidence):
                            self.validate_case(record)

    def test_rejects_invalid_sample_cancellation_fields(self) -> None:
        for field in ("cancellation_assertion_delay_cycles", "cancellation_latency_cycles"):
            for invalid in (None, True, -1, 1.5, "7", 0xFFFFFFFF, 0x100000000):
                with self.subTest(field=field, invalid=invalid):
                    record = case(0)
                    record["irq_loaded"][field][1] = invalid
                    with self.assertRaises(validate.InvalidEvidence):
                        self.validate_case(record)

    def test_rejects_inflated_duration_hiding_wrapped_latency(self) -> None:
        for form in ("sample", "cold", "stack_pattern_b"):
            with self.subTest(form=form):
                record = case(0)
                self.set_irq_interval(record, form, 100, 115, 117)
                loaded = record["irq_loaded"]
                if form == "sample":
                    loaded["samples"][0] = 0x100000000
                else:
                    loaded[f"{form}_cycles"] = 0x100000000
                loaded["max_cycles"] = 0x100000000
                with self.assertRaises(validate.InvalidEvidence):
                    self.validate_case(record)

    def test_requires_exact_cancellation_observation_count(self) -> None:
        invalid_counts = ((0, 0), (4, 4), (6, 6), (4, 5), (None, None), (True, True), (5.0, 5.0))
        for observed, expected in invalid_counts:
            with self.subTest(observed=observed, expected=expected):
                record = case(0)
                record["irq_loaded"]["cancellation_observed"] = observed
                record["irq_loaded"]["cancellation_expected"] = expected
                with self.assertRaisesRegex(validate.InvalidEvidence, "cancellation"):
                    self.validate_case(record)

    def test_summary_includes_cold_and_stack_b_cancellation_maxima(self) -> None:
        for form in ("cold", "stack_pattern_b"):
            with self.subTest(form=form):
                record = case(0)
                record["irq_loaded"][f"{form}_cancellation_assertion_delay_cycles"] = 20
                record["irq_loaded"][f"{form}_cancellation_latency_cycles"] = 12
                summary, passed = self.validate_case(record)
                self.assertTrue(passed)
                self.assertEqual(summary["cancellation_assertion_delay_max_cycles"], 20)
                self.assertEqual(summary["cancellation_latency_max_cycles"], 12)


if __name__ == "__main__":
    unittest.main()
