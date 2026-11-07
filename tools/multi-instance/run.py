#!/usr/bin/env python3
"""Run isolated release workloads serially, preserving raw JSONL and provenance."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def cases(suite):
    base = dict(population=1000, active=0.01, lanes=8, slots=1, members=16,
                rounds=3, mode="shared", driver="engine", accounts="independent", delay_us=0)
    variants = []
    if suite == "smoke":
        for mode in ("shared", "targeted", "cold"):
            for driver in ("engine", "binding"):
                variants.append(dict(population=16, active=1, lanes=2, mode=mode, driver=driver))
    elif suite == "baseline":
        variants = [dict(), dict(population=10000, slots=32), dict(mode="targeted")]
    elif suite == "targeted-load":
        variants = [dict(population=10000, active=1, mode="targeted")]
    else:
        for n in (1, 100, 1000, 10000):
            for active in (0, 0.01, 0.1, 1):
                variants.append(dict(population=n, active=active, lanes=min(n, 8)))
        for mode in ("targeted", "cold"):
            for n in (100, 1000, 10000):
                variants.append(dict(mode=mode, population=n))
        for lanes in (1, 8, 32):
            for driver in ("engine", "binding"):
                for delay in (0, 1000):
                    variants.append(dict(active=1, lanes=lanes, driver=driver, delay_us=delay))
        variants += [dict(population=10000, slots=32),
                     dict(population=10000, accounts="shared"),
                     dict(population=10000, accounts="shared", slots=32)]
        variants += [dict(mode="targeted", members=n) for n in (1, 29)]
        variants += [dict(population=1, lanes=1, mode="targeted", members=n, active=0) for n in (30, 32)]
        variants += [dict(population=n, mode="targeted", accounts="shared", active=0) for n in (896, 897)]
        variants.append(dict(population=10000, active=1, mode="targeted"))
    unique = {}
    for variant in variants:
        case = dict(base, **variant)
        unique[json.dumps(case, sort_keys=True)] = case
    return list(unique.values())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite", choices=("smoke", "baseline", "targeted-load", "full"), default="full")
    parser.add_argument("--repeat", type=int, default=3)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/examples/multi_instance")
    parser.add_argument("--allow-dirty", action="store_true")
    args = parser.parse_args()
    assert args.repeat > 0
    status = command("git", "status", "--porcelain")
    if status and not args.allow_dirty:
        parser.error("commit the measured sources first, or explicitly use --allow-dirty")
    args.output.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    env = dict(revision=command("git", "rev-parse", "HEAD"), status=status,
               diff_sha256=hashlib.sha256(subprocess.check_output(["git", "diff", "HEAD"], cwd=ROOT)).hexdigest(),
               binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
               cargo_lock_sha256=hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest(),
               rustc=command("rustc", "-Vv"), kernel=platform.platform(),
               cpu=next((s.split(":", 1)[1].strip() for s in Path("/proc/cpuinfo").read_text().splitlines() if s.startswith("model name")), "unknown"),
               affinity=sorted(os.sched_getaffinity(0)), libc=platform.libc_ver(),
               started_utc=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
               suite=args.suite, repeat=args.repeat, build="cargo build --release --locked --offline -p clinkz-wot-binding-http --example multi_instance",
               runner_argv=list(os.sys.argv), cases=cases(args.suite))
    (args.output / "environment.json").write_text(json.dumps(env, indent=2) + "\n")
    with (args.output / "measurements.jsonl").open("x") as output:
        for repeat in range(args.repeat):
            rows = list(enumerate(env["cases"]))
            if repeat % 2:
                rows.reverse()
            for number, case in rows:
                flags = [str(binary)]
                for k, v in case.items():
                    flags += ["--" + k.replace("_", "-"), str(v)]
                run = subprocess.run(flags, cwd=ROOT, capture_output=True, text=True, timeout=240)
                if run.returncode:
                    raise RuntimeError(f"case {case}: {run.stderr}\n{run.stdout}")
                phases = [json.loads(line) for line in run.stdout.splitlines()]
                assert phases[-1]["phase"] == "release_accounts"
                refused = any(p["phase"] == "preparation_refused" for p in phases)
                expected_refusal = case["members"] >= 30 or (case["accounts"] == "shared" and case["mode"] != "shared" and case["population"] > 896)
                assert refused == expected_refusal, f"unexpected preparation disposition: {case}"
                for row in phases:
                    output.write(json.dumps(dict(case=number, repeat=repeat, **row), separators=(",", ":")) + "\n")
                output.flush()
                print(f"replicate {repeat + 1}/{args.repeat} case {number + 1}/{len(rows)}: {case}", flush=True)


if __name__ == "__main__":
    main()
