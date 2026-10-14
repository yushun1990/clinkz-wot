#!/usr/bin/env python3
"""Compile disposable probes against the repository lock, without a new crate."""
import json
import pathlib
import resource
import signal
import subprocess
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
build = subprocess.run(
    ["cargo", "build", "--locked", "--offline", "-p",
     "clinkz-wot-protocol-bindings-zenoh", "-p", "clinkz-wot-foundation",
     "--message-format=json"],
    cwd=ROOT, check=True, text=True, stdout=subprocess.PIPE,
)
artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith("{")]
dependencies = {}
for name in ["zenoh", "tokio", "clinkz_wot_foundation"]:
    dependencies[name] = next(
        pathlib.Path(path) for a in artifacts
        if a.get("reason") == "compiler-artifact" and a["target"]["name"] == name
        for path in a["filenames"] if path.endswith(".rlib")
    )
with tempfile.TemporaryDirectory(prefix="wp600-owner-") as directory:
    binary = pathlib.Path(directory) / "probe"
    command = ["rustc", "--edition=2024", str(HERE / "probe.rs"), "-o", str(binary),
               "-L", f"dependency={dependencies['zenoh'].parent}"]
    for name, path in dependencies.items():
        command.extend(["--extern", f"{name}={path}"])
    subprocess.run(command, cwd=ROOT, check=True)
    for case in ["last-drop", "abandoned-close", "get-callback", "separate-owner", "closed-drop"]:
        subprocess.run([str(binary), case], cwd=ROOT, check=True, timeout=15)
    # Fault injection affects only the thread performing final Session Drop.
    # Disable core dumps before the deliberately aborting child is started.
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    failed = subprocess.run([str(binary), "closed-drop-oom"], cwd=ROOT,
                            text=True, capture_output=True, timeout=15)
    assert failed.returncode == -signal.SIGABRT, failed
    assert "memory allocation of" in failed.stderr, failed.stderr
    print("closed-session-drop-oom: controlled allocation failure aborts child (SIGABRT)")
