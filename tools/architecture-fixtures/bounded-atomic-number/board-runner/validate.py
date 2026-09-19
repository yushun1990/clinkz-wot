#!/usr/bin/env python3
"""Validate completeness and summarize WP100-ATOMIC-NUMBER-M4-v1 raw JSONL."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path


EXPECTED_SCHEMA = "wp100-atomic-number-m4-raw-v1"
EXPECTED_WORKLOAD = "WP100-ATOMIC-NUMBER-M4-v1"
EXPECTED_CASES = 12_844
EXPECTED_SAMPLES = 1_000
EXPECTED_WARMUPS = 100
EXPECTED_CYCLE_LIMIT = 168_000
EXPECTED_STACK_LIMIT = 4_096


class InvalidEvidence(Exception):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise InvalidEvidence(message)


def is_integer(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def nonnegative_integer(value: object, context: str) -> int:
    require(is_integer(value) and value >= 0, f"{context} is not a nonnegative integer")
    return value


def cycle_sample(value: object, context: str) -> int:
    value = nonnegative_integer(value, context)
    # Firmware uses u32::MAX as a missing-measurement sentinel (null in arrays).
    require(value < 0xFFFFFFFF, f"{context} is not a valid u32 cycle sample")
    return value


def integer_samples(value: object, context: str) -> list[int]:
    require(isinstance(value, list), f"{context} is not an array")
    require(len(value) == EXPECTED_SAMPLES, f"{context} has {len(value)} samples")
    require(
        all(is_integer(sample) and 0 <= sample < 0xFFFFFFFF for sample in value),
        f"{context} contains a missing or invalid sample",
    )
    return value


def require_cancellation_in_interval(cycles: int, latency: int, context: str) -> None:
    # With the workload's single-wrap interval, (stop - irq) mod 2^32 must
    # fit inside (stop - start) mod 2^32. An IRQ before start exceeds cycles;
    # one after stop wraps to a large value. Observation alone proves neither.
    require(latency <= cycles, f"{context} cancellation is outside the measured projection interval")


def load_record(line: str, line_number: int) -> dict[str, object]:
    try:
        value = json.loads(line)
    except json.JSONDecodeError as error:
        raise InvalidEvidence(f"line {line_number} is not JSON: {error}") from error
    require(isinstance(value, dict), f"line {line_number} is not a JSON object")
    return value


def validate(raw_path: Path, coverage_path: Path | None) -> tuple[dict[str, object], bool]:
    digest = hashlib.sha256()
    lengths: set[int] = set()
    families: set[str] = set()
    case_count = 0
    reported_failed_cases = 0
    masked_cycle_max = 0
    masked_stack_max = 0
    irq_cycle_max = 0
    cancellation_latency_max = 0
    cancellation_assertion_delay_max = 0
    interrupt_stack_max = 0
    run_start: dict[str, object] | None = None
    run_end: dict[str, object] | None = None

    with raw_path.open("rb") as binary:
        for line_number, encoded in enumerate(binary, start=1):
            digest.update(encoded)
            try:
                line = encoded.decode("utf-8")
            except UnicodeDecodeError as error:
                raise InvalidEvidence(f"line {line_number} is not UTF-8") from error
            require(line.endswith("\n"), f"line {line_number} is truncated")
            record = load_record(line, line_number)
            kind = record.get("record")

            if kind == "run_start":
                require(run_start is None and case_count == 0, "run_start is duplicated or misplaced")
                require(record.get("schema") == EXPECTED_SCHEMA, "raw schema mismatch")
                require(record.get("workload_id") == EXPECTED_WORKLOAD, "workload mismatch")
                require(record.get("expected_cases") == EXPECTED_CASES, "expected case count mismatch")
                require(record.get("warmup_iterations") == EXPECTED_WARMUPS, "warmup count mismatch")
                require(record.get("sample_count") == EXPECTED_SAMPLES, "sample count mismatch")
                require(record.get("cycle_limit") == EXPECTED_CYCLE_LIMIT, "cycle limit mismatch")
                require(record.get("stack_limit_bytes") == EXPECTED_STACK_LIMIT, "stack limit mismatch")
                require(record.get("target") == "thumbv7em-none-eabihf", "target mismatch")
                require(record.get("board") == "STM32F407G-DISC1", "board mismatch")
                require(
                    record.get("cpu") == "STM32F407VGT6 Cortex-M4F r0p1",
                    "CPU declaration mismatch",
                )
                require(record.get("cpuid") == 0x410FC241, "Cortex-M4F CPUID mismatch")
                device_idcode = nonnegative_integer(record.get("device_idcode"), "device IDCODE")
                require(device_idcode & 0xFFF == 0x413, "STM32F407 device ID mismatch")
                require(record.get("clock_hz") == 168_000_000, "clock declaration mismatch")
                require(record.get("clock_source") == "8MHz-HSE-PLL", "clock source mismatch")
                require(record.get("flash_wait_states") == 5, "FLASH wait-state declaration mismatch")
                require(record.get("flash_prefetch") is True, "FLASH prefetch declaration mismatch")
                require(
                    record.get("flash_instruction_cache") is True,
                    "FLASH instruction-cache declaration mismatch",
                )
                require(record.get("flash_data_cache") is True, "FLASH data-cache declaration mismatch")
                require(record.get("code_placement") == "flash", "code placement mismatch")
                require(record.get("project_stack_placement") == "sram", "project stack placement mismatch")
                require(record.get("main_stack_placement") == "ccm", "main stack placement mismatch")
                require(
                    record.get("allocator") == "first-fit-coalescing-static-48k-sram-v1",
                    "allocator declaration mismatch",
                )
                require(record.get("masked_run_interrupts") == "PRIMASK=1", "masked IRQ declaration mismatch")
                require(
                    record.get("application_irq_load")
                    == "one TIM2 update IRQ per projection, 84MHz timer, ARR=32, ISR asserts cancellation and timestamps entry/exit",
                    "application IRQ declaration mismatch",
                )
                rcc_cr = nonnegative_integer(record.get("rcc_cr"), "RCC CR")
                rcc_pllcfgr = nonnegative_integer(record.get("rcc_pllcfgr"), "RCC PLLCFGR")
                rcc_cfgr = nonnegative_integer(record.get("rcc_cfgr"), "RCC CFGR")
                flash_acr = nonnegative_integer(record.get("flash_acr"), "FLASH ACR")
                require(
                    rcc_cr & ((1 << 16) | (1 << 17) | (1 << 24) | (1 << 25))
                    == ((1 << 16) | (1 << 17) | (1 << 24) | (1 << 25)),
                    "HSE/PLL is not enabled and ready",
                )
                require(rcc_pllcfgr & 0x3F == 8, "PLLM is not 8")
                require((rcc_pllcfgr >> 6) & 0x1FF == 336, "PLLN is not 336")
                require((rcc_pllcfgr >> 16) & 0x3 == 0, "PLLP is not 2")
                require(rcc_pllcfgr & (1 << 22) != 0, "PLL source is not HSE")
                require((rcc_pllcfgr >> 24) & 0xF == 7, "PLLQ is not 7")
                require(rcc_cfgr & 0x3 == 0x2, "system clock request is not PLL")
                require((rcc_cfgr >> 2) & 0x3 == 0x2, "system clock status is not PLL")
                require((rcc_cfgr >> 4) & 0xF == 0, "AHB prescaler is not 1")
                require((rcc_cfgr >> 10) & 0x7 == 0b101, "APB1 prescaler is not 4")
                require((rcc_cfgr >> 13) & 0x7 == 0b100, "APB2 prescaler is not 2")
                require(flash_acr & 0x7 == 5, "FLASH latency is not five wait states")
                require(
                    flash_acr & ((1 << 8) | (1 << 9) | (1 << 10))
                    == ((1 << 8) | (1 << 9) | (1 << 10)),
                    "FLASH prefetch/instruction/data caches are not enabled",
                )
                run_start = record
                continue

            if kind == "case":
                require(run_start is not None and run_end is None, "case is outside run boundaries")
                require(record.get("case_index") == case_count, f"case index {case_count} is missing")
                lexeme = record.get("lexeme")
                family = record.get("family")
                require(isinstance(lexeme, str), f"case {case_count} has no lexeme")
                require(isinstance(family, str) and family, f"case {case_count} has no family")
                require(record.get("length") == len(lexeme), f"case {case_count} length mismatch")
                require(1 <= len(lexeme) <= 256, f"case {case_count} is outside lexical bound")
                require(isinstance(record.get("projection"), dict), f"case {case_count} has no projection")
                projection = record["projection"]
                projection_kind = projection.get("kind")
                if projection_kind == "finite":
                    bits = projection.get("bits")
                    require(
                        isinstance(bits, str) and re.fullmatch(r"0x[0-9a-f]{16}", bits) is not None,
                        f"case {case_count} has invalid projection bits",
                    )
                else:
                    require(projection_kind == "rejection", f"case {case_count} projection kind is invalid")
                    require(
                        projection.get("value")
                        in {"Limit", "InvalidSchema", "InvalidConfiguration"},
                        f"case {case_count} rejection is invalid",
                    )
                lengths.add(len(lexeme))
                families.add(family)

                masked = record.get("masked")
                irq = record.get("irq_loaded")
                require(isinstance(masked, dict), f"case {case_count} has no masked result")
                require(isinstance(irq, dict), f"case {case_count} has no IRQ-loaded result")
                masked_samples = integer_samples(masked.get("samples"), f"case {case_count} masked samples")
                irq_samples = integer_samples(irq.get("samples"), f"case {case_count} IRQ samples")
                assertion_delays = integer_samples(
                    irq.get("cancellation_assertion_delay_cycles"),
                    f"case {case_count} assertion delays",
                )
                cancellation_latencies = integer_samples(
                    irq.get("cancellation_latency_cycles"),
                    f"case {case_count} cancellation latencies",
                )
                for sample_index, (cycles, latency) in enumerate(zip(irq_samples, cancellation_latencies)):
                    require_cancellation_in_interval(
                        cycles, latency, f"case {case_count} IRQ sample {sample_index}"
                    )

                cold = cycle_sample(masked.get("cold_cycles"), f"case {case_count} masked cold sample")
                stack_b_cycles = cycle_sample(
                    masked.get("stack_pattern_b_cycles"), f"case {case_count} masked stack-B sample"
                )
                actual_masked_max = max(cold, stack_b_cycles, *masked_samples)
                require(masked.get("max_cycles") == actual_masked_max, f"case {case_count} masked max mismatch")
                require(masked.get("allocation_calls") == 0, f"case {case_count} allocated while masked")
                stack_a = masked.get("stack_pattern_a_depth_bytes")
                stack_b = masked.get("stack_pattern_b_depth_bytes")
                stack_max = masked.get("stack_watermark_conservative_depth_bytes")
                require(
                    all(is_integer(value) and value >= 0 for value in (stack_a, stack_b, stack_max)),
                    f"case {case_count} has incomplete masked stack data",
                )
                require(stack_max == max(stack_a, stack_b), f"case {case_count} stack max mismatch")

                require(irq.get("allocation_calls") == 0, f"case {case_count} allocated with IRQ load")
                expected_cancellations = EXPECTED_WARMUPS + EXPECTED_SAMPLES + 2
                require(
                    all(
                        is_integer(irq.get(field)) and irq[field] == expected_cancellations
                        for field in ("cancellation_observed", "cancellation_expected")
                    ),
                    f"case {case_count} cancellation observation count mismatch",
                )
                require(record.get("result_mismatches") == 0, f"case {case_count} result mismatch")
                require(record.get("guard_ok") is True, f"case {case_count} damaged a stack guard")
                require(
                    record.get("stack_watermark_certain") is True,
                    f"case {case_count} has an uncertain stack watermark",
                )

                irq_cold = cycle_sample(
                    irq.get("cold_cycles"), f"case {case_count} IRQ cold sample"
                )
                irq_stack_b_cycles = cycle_sample(
                    irq.get("stack_pattern_b_cycles"), f"case {case_count} IRQ stack-B sample"
                )
                actual_irq_max = max(irq_cold, irq_stack_b_cycles, *irq_samples)
                require(irq.get("max_cycles") == actual_irq_max, f"case {case_count} IRQ max mismatch")
                for form, cycles in (("cold", irq_cold), ("stack_pattern_b", irq_stack_b_cycles)):
                    context = f"case {case_count} IRQ {form}"
                    delay = cycle_sample(
                        irq.get(f"{form}_cancellation_assertion_delay_cycles"), f"{context} assertion delay"
                    )
                    latency = cycle_sample(
                        irq.get(f"{form}_cancellation_latency_cycles"), f"{context} cancellation latency"
                    )
                    require_cancellation_in_interval(cycles, latency, context)
                    assertion_delays.append(delay)
                    cancellation_latencies.append(latency)

                masked_cycle_max = max(masked_cycle_max, actual_masked_max)
                masked_stack_max = max(masked_stack_max, stack_max)
                irq_cycle_max = max(irq_cycle_max, actual_irq_max)
                cancellation_latency_max = max(cancellation_latency_max, max(cancellation_latencies))
                cancellation_assertion_delay_max = max(
                    cancellation_assertion_delay_max, max(assertion_delays)
                )
                irq_project_a = irq.get("stack_pattern_a_project_depth_bytes")
                irq_project_b = irq.get("stack_pattern_b_project_depth_bytes")
                irq_stack_a = irq.get("stack_pattern_a_interrupt_depth_bytes")
                irq_stack_b = irq.get("stack_pattern_b_interrupt_depth_bytes")
                require(
                    all(
                        is_integer(value) and value >= 0
                        for value in (irq_project_a, irq_project_b, irq_stack_a, irq_stack_b)
                    ),
                    f"case {case_count} has incomplete IRQ stack data",
                )
                interrupt_stack = irq.get("interrupt_stack_watermark_conservative_depth_bytes")
                require(
                    interrupt_stack == max(irq_stack_a, irq_stack_b),
                    f"case {case_count} interrupt stack max mismatch",
                )
                interrupt_stack_max = max(interrupt_stack_max, interrupt_stack)

                irq_project_stack = irq.get("project_stack_watermark_conservative_depth_bytes")
                require(
                    irq_project_stack == max(irq_project_a, irq_project_b),
                    f"case {case_count} IRQ project stack max mismatch",
                )

                expected_failure = (
                    actual_masked_max > EXPECTED_CYCLE_LIMIT
                    or stack_max > EXPECTED_STACK_LIMIT
                    or irq_project_stack > EXPECTED_STACK_LIMIT
                )
                require(record.get("failed") == expected_failure, f"case {case_count} failure flag mismatch")
                reported_failed_cases += int(expected_failure)
                case_count += 1
                continue

            if kind == "run_end":
                require(run_start is not None and run_end is None, "run_end is duplicated or misplaced")
                run_end = record
                continue

            raise InvalidEvidence(f"line {line_number} has unknown record kind {kind!r}")

    require(run_start is not None, "run_start is missing")
    require(run_end is not None, "run_end is missing")
    require(case_count == EXPECTED_CASES, f"expected {EXPECTED_CASES} cases, found {case_count}")
    require(lengths == set(range(1, 257)), "corpus does not cover every length 1..256")
    require(run_end.get("workload_id") == EXPECTED_WORKLOAD, "run_end workload mismatch")
    require(run_end.get("case_count") == case_count, "run_end case count mismatch")
    require(run_end.get("failed_cases") == reported_failed_cases, "run_end failed-case count mismatch")
    require(run_end.get("complete") is True, "firmware did not mark the run complete")

    coverage_confirmed = False
    coverage_sha256 = None
    if coverage_path is not None:
        coverage = coverage_path.read_bytes()
        coverage_sha256 = hashlib.sha256(coverage).hexdigest()
        coverage_text = coverage.decode("utf-8", errors="replace")
        coverage_confirmed = "parse_long_mantissa" in coverage_text or "dec2flt/slow.rs" in coverage_text
        require(coverage_confirmed, "coverage trace does not show the slow fallback")

    accepted_bounds = reported_failed_cases == 0
    summary: dict[str, object] = {
        "schema": "wp100-atomic-number-m4-summary-v1",
        "workload_id": EXPECTED_WORKLOAD,
        "raw_sha256": digest.hexdigest(),
        "coverage_sha256": coverage_sha256,
        "coverage_slow_fallback_confirmed": coverage_confirmed,
        "case_count": case_count,
        "family_count": len(families),
        "samples_per_case": EXPECTED_SAMPLES,
        "masked_cycle_max": masked_cycle_max,
        "masked_stack_watermark_max_bytes": masked_stack_max,
        "irq_loaded_cycle_max": irq_cycle_max,
        "cancellation_assertion_delay_max_cycles": cancellation_assertion_delay_max,
        "cancellation_latency_max_cycles": cancellation_latency_max,
        "interrupt_stack_watermark_max_bytes": interrupt_stack_max,
        "failed_cases": reported_failed_cases,
        "measurement_bounds_pass": accepted_bounds,
        "status": "candidate-passed" if accepted_bounds and coverage_confirmed else "candidate-failed",
        "admission_claim": False,
    }
    return summary, accepted_bounds and coverage_confirmed


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("raw", type=Path)
    parser.add_argument("--coverage", type=Path)
    parser.add_argument("--summary", type=Path)
    arguments = parser.parse_args()

    try:
        summary, passed = validate(arguments.raw, arguments.coverage)
    except (InvalidEvidence, OSError) as error:
        print(f"invalid evidence: {error}", file=sys.stderr)
        return 1

    rendered = json.dumps(summary, indent=2, sort_keys=True) + "\n"
    if arguments.summary is not None:
        arguments.summary.write_text(rendered, encoding="utf-8")
    sys.stdout.write(rendered)
    return 0 if passed else 2


if __name__ == "__main__":
    raise SystemExit(main())
