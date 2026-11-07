#!/usr/bin/env python3
"""Summarize a run directory (raw JSONL may be gzip-compressed) as median CSV."""
import argparse
import csv
import gzip
import json
from pathlib import Path
import statistics
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    args = parser.parse_args()
    environment = json.loads((args.run / "environment.json").read_text())
    path = args.run / "measurements.jsonl"
    opener = open
    if not path.exists():
        path = path.with_suffix(".jsonl.gz")
        opener = gzip.open
    runs = {}
    with opener(path, "rt") as source:
        for line in source:
            record = json.loads(line)
            runs.setdefault((record["case"], record["repeat"]), {})[record["phase"]] = record
    output = []
    for number, case in enumerate(environment["cases"]):
        samples = [runs[(number, repeat)] for repeat in range(environment["repeat"])]
        median = lambda fn: statistics.median(fn(sample) for sample in samples)
        config = samples[0]["configuration"]
        if "preparation_refused" in samples[0]:
            output.append(dict(case=number, **case, status="preparation_refused",
                native_jobs=0, content_constructions=0,
                preparation_ms=median(lambda s: s["preparation_refused"]["ns"]) / 1e6,
                final_bank_reservations_zero=all(all(v["logical_and_mapping"] == v["physical"] == 0 for v in s["refusal_release"]["reservations"].values()) for s in samples)))
            assert all(s["preparation_refused"]["native_jobs"] == 0 and s["peer_final"]["stats"]["requests"] == 0 for s in samples)
            continue
        calls = config["active"] * case["rounds"]
        engine = case["driver"] == "engine"
        def retained(sample):
            return sample["admitted_calls"]["net_live_bytes"] + sample["retained_results"]["net_live_bytes"]
        rows = dict(case=number, **case, status="completed", active_count=config["active"], completed=calls,
            preparation_ms=median(lambda s: s["preparation"]["ns"]) / 1e6,
            parse_ms=median(lambda s: s["preparation"]["parse"]["ns"]) / 1e6,
            qualification_ms=median(lambda s: s["preparation"]["qualification_including_native"]["ns"]) / 1e6,
            native_step_ms=median(lambda s: s["preparation"]["native_job_steps"]["ns"]) / 1e6,
            preparation_allocations=median(lambda s: s["preparation"]["allocation_calls"]),
            preparation_requested_bytes=median(lambda s: s["preparation"]["requested_bytes"]),
            preparation_retained_bytes=median(lambda s: s["preparation"]["net_live_bytes"]),
            preparation_peak_above_start_bytes=median(lambda s: s["preparation"]["heap_peak_above_start_bytes"]),
            content_constructions=config["native_mappings"] if case["mode"] == "cold" else 1,
            native_jobs=config["native_mappings"],
            idle_bind_ms=median(lambda s: s["idle_instances"]["ns"]) / 1e6,
            idle_instance_bytes=median(lambda s: s["idle_instances"]["net_live_bytes"]) / case["population"],
            idle_instance_allocations=median(lambda s: s["idle_instances"]["allocation_calls"]) / case["population"],
            account_provisioning_bytes=median(lambda s: s["account_provisioning"]["net_live_bytes"]),
            idle_rss_bytes=median(lambda s: s["idle_instances"]["os"]["rss_bytes"]),
            idle_glibc_in_use_bytes=median(lambda s: s["idle_instances"]["os"]["glibc"]["in_use_bytes"]),
            throughput_per_s=median(lambda s: s["load"]["throughput_per_s"]),
            throughput_min=min(s["load"]["throughput_per_s"] for s in samples),
            throughput_max=max(s["load"]["throughput_per_s"] for s in samples),
            service_p50_us=median(lambda s: s["load"]["admitted_latency"]["p50_ns"] or 0) / 1000,
            service_p95_us=median(lambda s: s["load"]["admitted_latency"]["p95_ns"] or 0) / 1000,
            service_p99_us=median(lambda s: s["load"]["admitted_latency"]["p99_ns"] or 0) / 1000,
            cohort_p99_ms=median(lambda s: s["load"]["cohort_latency"]["p99_ns"] or 0) / 1e6,
            load_allocations_per_call=median(lambda s: s["load"]["allocation_calls"]) / calls if calls else 0,
            load_requested_bytes_per_call=median(lambda s: s["load"]["requested_bytes"]) / calls if calls else 0,
            admission_cpu_us_per_call=median(lambda s: s["load"]["admission_cpu_ns"]) / (calls * 1000) if calls else 0,
            progress_cpu_us_per_call=median(lambda s: s["load"]["progress_cpu_ns"]) / (calls * 1000) if calls else 0,
            admitted_heap_bytes_per_lane=median(lambda s: s["admitted_calls"]["net_live_bytes"]) / case["lanes"] if engine else "",
            result_heap_bytes_per_lane=median(retained) / case["lanes"] if engine else "",
            active_reservation_bytes_per_lane=median(lambda s: s["admitted_calls"]["reservations"]["EngineLiveBytesGlobalMax"]["logical_and_mapping"] - s["idle_instances"]["reservations"]["EngineLiveBytesGlobalMax"]["logical_and_mapping"]) / case["lanes"] if engine else "",
            cancellation_us=median(lambda s: s["lifecycle"]["cancellation_ns"]) / 1000 if engine else "",
            replacement_us=median(lambda s: s["lifecycle"]["replacement_ns"]) / 1000 if engine else "",
            held_result_blocks_sibling=any(s["lifecycle"]["held_result_blocks_sibling"] for s in samples) if engine else "",
            release_result_us=median(lambda s: s["release_results"]["ns"]) / 1000,
            final_bank_reservations_zero=all(all(v["logical_and_mapping"] == v["physical"] == 0 for v in s["release_results"]["reservations"].values()) for s in samples))
        assert all(s["preparation"]["native_jobs"] == rows["native_jobs"] for s in samples)
        assert all(s["preparation"]["content_constructions"] == rows["content_constructions"] for s in samples)
        assert all(s["load"]["completed"] == calls for s in samples)
        output.append(rows)
    writer = csv.DictWriter(sys.stdout, fieldnames=list(output[0]))
    writer.writeheader()
    writer.writerows(output)


if __name__ == "__main__":
    main()
