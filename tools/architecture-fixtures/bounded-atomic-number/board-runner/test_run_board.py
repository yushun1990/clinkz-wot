#!/usr/bin/env python3
"""Host tests for the probe-rs 0.32.0 board command sequence."""

from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path


RUNNER = Path(__file__).with_name("run-board.sh")
PROBE_VERSION = "probe-rs 0.32.0 (git commit: 48f5e4d)"


class BoardRunnerTests(unittest.TestCase):
    def make_environment(self, root: Path) -> tuple[dict[str, str], Path]:
        fake_bin = root / "bin"
        fake_bin.mkdir()
        log = root / "probe-rs.log"

        probe_rs = fake_bin / "probe-rs"
        probe_rs.write_text(
            """#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$*" >> "$WP100_TEST_PROBE_LOG"
case "${1:-}" in
    --version)
        printf '%s\\n' 'probe-rs 0.32.0 (git commit: 48f5e4d)'
        ;;
    gdb)
        printf '%s\\n' 'Breakpoint 1, core::num::dec2flt::slow::parse_long_mantissa<f64> () at library/core/src/num/dec2flt/slow.rs:39'
        printf '%s\\n' 'WP100_SLOW_FALLBACK_HIT'
        printf '%s\\n' '#0 core::num::dec2flt::slow::parse_long_mantissa<f64> at library/core/src/num/dec2flt/slow.rs:39'
        ;;
esac
""",
            encoding="utf-8",
        )
        probe_rs.chmod(0o755)

        gdb = fake_bin / "gdb"
        gdb.write_text(
            """#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--version" ]]; then
    printf '%s\\n' 'GNU gdb test-double'
else
    printf '%s\\n' 'Breakpoint 1 at 0x080144b0 in core::num::dec2flt::slow::parse_long_mantissa<f64> at library/core/src/num/dec2flt/slow.rs:39'
fi
""",
            encoding="utf-8",
        )
        gdb.chmod(0o755)

        environment = os.environ.copy()
        environment["PATH"] = f"{fake_bin}:{environment['PATH']}"
        environment["WP100_TEST_PROBE_LOG"] = str(log)
        environment["WP100_PROBE"] = "0483:374b:TEST"
        environment["WP100_COVERAGE_SECONDS"] = "17"
        return environment, log

    def make_artifact(self, root: Path) -> Path:
        artifact = root / "artifacts"
        artifact.mkdir()
        (artifact / "firmware.elf").write_bytes(b"test ELF")
        (artifact / "probe-rs-version.txt").write_text(PROBE_VERSION + "\n", encoding="utf-8")
        return artifact

    def test_coverage_downloads_then_resets_then_attaches_debugger(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            environment, log = self.make_environment(root)
            artifact = self.make_artifact(root)

            result = subprocess.run(
                [str(RUNNER), "coverage", str(artifact)],
                input="\n",
                text=True,
                capture_output=True,
                env=environment,
                check=False,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            invocations = log.read_text(encoding="utf-8").splitlines()
            actions = [line.split()[0] for line in invocations if not line.startswith("--version")]
            self.assertEqual(actions, ["download", "reset", "gdb"])
            download = next(line for line in invocations if line.startswith("download "))
            debugger = next(line for line in invocations if line.startswith("gdb "))
            self.assertIn("--verify", download)
            self.assertNotIn("--reset", download)
            self.assertNotIn("--flash", debugger)
            self.assertNotIn("--reset", debugger)

            trace = (artifact / "slow-fallback-coverage.txt").read_text(encoding="utf-8")
            self.assertIn("WP100_SLOW_FALLBACK_HIT", trace)
            self.assertIn("parse_long_mantissa", trace)

            commands = (artifact / "commands.txt").read_text(encoding="utf-8")
            self.assertNotIn("probe-rs profile", commands)
            self.assertLess(commands.index("probe-rs download"), commands.index("probe-rs reset"))
            self.assertLess(commands.index("probe-rs reset"), commands.index("probe-rs gdb"))
            self.assertIn("-- --batch", commands)
            self.assertIn("WP100_SLOW_FALLBACK_HIT", commands)
            syntax = subprocess.run(
                ["bash", "-n", str(artifact / "commands.txt")],
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(syntax.returncode, 0, syntax.stderr)

    def test_measure_rejects_an_unconfirmed_stale_coverage_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            environment, log = self.make_environment(root)
            artifact = self.make_artifact(root)
            (artifact / "slow-fallback-coverage.txt").write_text(
                "error: the following required argument was not provided: reset\n",
                encoding="utf-8",
            )

            result = subprocess.run(
                [str(RUNNER), "measure", str(artifact)],
                text=True,
                capture_output=True,
                env=environment,
                check=False,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("confirmed slow-fallback breakpoint hit", result.stderr)
            invocations = log.read_text(encoding="utf-8").splitlines()
            self.assertEqual(invocations, ["--version"])


if __name__ == "__main__":
    unittest.main()
