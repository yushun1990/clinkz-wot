#!/usr/bin/env python3
"""Retained production predebit counterexamples, executed on native and ARM.

Only disposable source copies are mutated. A compile failure is never evidence
of rejection: each case must build, then fail through the intended access/work
oracle. Zero-credit and compensated cases preserve complete-run facts/charges.
"""
import hashlib
import os
from pathlib import Path
import resource
import shutil
import signal
import subprocess
import sys
import tempfile


def replace_once(source, before, after):
    if source.count(before) != 1:
        raise RuntimeError("production mutation site changed; update its counterexample")
    return source.replace(before, after, 1)


def cases(source, uri_source):
    allocation = """            P::Allocate => {
                self.allocate_uri()?;
                self.phase = P::Uri;
            }"""
    for name, condition in [
        ("acquisition-copy", "true"),
        ("zero-output-acquisition-copy", "!replacement && budget.remaining(W::CodecOutputBytes) == 0"),
        ("zero-output-replacement-copy", "replacement && budget.remaining(W::CodecOutputBytes) == 0"),
    ]:
        injected = """            P::Allocate => {
                let replacement = self.scratch.capacity != 0;
                self.allocate_uri()?;
                if CONDITION {
                    let byte = core::hint::black_box(self.current_form().href.as_str().as_bytes()[0]);
                    unsafe { self.scratch.pointer.write_volatile(byte); }
                }
                self.phase = P::Uri;
            }""".replace("CONDITION", condition)
        if condition == "true":
            injected = injected.replace("                let replacement = self.scratch.capacity != 0;\n", "")
        yield name, replace_once(source, allocation, injected), uri_source, False
    work = "self.resolver.as_ref().unwrap().work()"
    for name, condition in [
        ("arm-missing-uri-charge", 'cfg!(target_os = "none")'),
        ("arm-zero-credit-missing-uri-charge", 'cfg!(target_os = "none") && budget.remaining(W::UriBytes) == 0'),
    ]:
        yield name, replace_once(source, work, f"if {condition} {{ 0 }} else {{ {work} }}"), uri_source, True
    # Normal full-credit runs and zero-credit guards are unchanged. Only an
    # independently selected positive shortage can expose this undercharge.
    yield "positive-uri-short-only", replace_once(source, work,
        f"if budget.remaining(W::UriBytes) == 15 && {work} == 16 {{ 15 }} else {{ {work} }}"), uri_source, False
    # Undercharge each two-byte repair, then recover both missing units from
    # the first two tail bytes. Per-Form/lifetime totals and zero-credit probes
    # cannot distinguish this from a correctly prepaid run.
    tail = "                State::Tail if self.pos < self.tail[self.part].len() => 1,"
    compensated = replace_once(uri_source, tail, """                State::Tail
                    if self.part < 2 && self.pos == 0 && self.merge == self.path_start => 2,
""" + tail)
    compensated = replace_once(compensated,
                               "                State::ExtendPath | State::Insert => Self::OUTPUT_WORK,",
                               "                State::ExtendPath | State::Insert => 1,")
    yield "compensated-repair-copy-charge", source, compensated, False


def main():
    repo = Path(__file__).resolve().parents[3]
    cargo_args = sys.argv[1:]
    label = hashlib.sha256(repr(cargo_args).encode()).hexdigest()[:8]
    artifacts = repo / "target" / "td-admission-mutations"
    logs = artifacts / "logs" / label
    logs.mkdir(parents=True, exist_ok=True)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    versions = subprocess.check_output(["rustc", "-vV"], text=True)
    native = next(line.removeprefix("host: ") for line in versions.splitlines() if line.startswith("host: "))
    arm = "thumbv7em-none-eabihf"
    qemu = os.environ.get("QEMU_SYSTEM_ARM", "qemu-system-arm")
    print(f"production mutation controls: native={native} ARM={arm} cargo_args={cargo_args}", flush=True)

    with tempfile.TemporaryDirectory(prefix="td-predebit-mutations-") as directory:
        snapshot = Path(directory)
        # Copy current worktree contents, including uncommitted runtime repairs.
        # All workspace manifests/build inputs are retained; target/git caches
        # and unrelated untracked files are excluded.
        names = subprocess.check_output(["git", "ls-files", "-z"], cwd=repo).split(b"\0")
        for name in filter(None, names):
            path = Path(os.fsdecode(name))
            destination = snapshot / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(repo / path, destination)
        shutil.copytree(repo / "td/tests/runtime", snapshot / "td/tests/runtime", dirs_exist_ok=True,
                        ignore=shutil.ignore_patterns("target", "__pycache__"))
        production = snapshot / "td/src/validated.rs"
        uri = snapshot / "td/src/core/data_type/uri.rs"
        source = production.read_text()
        uri_source = uri.read_text()
        for name, mutated, mutated_uri, arm_only in [
            ("baseline", source, uri_source, False), *cases(source, uri_source)
        ]:
            production.write_text(mutated)
            uri.write_text(mutated_uri)
            for target in [native, arm]:
                log = logs / f"{name}-{target}.log"
                with log.open("w") as output:
                    command = ["cargo", "build", "--locked", "--release", "--manifest-path",
                               str(snapshot / "td/tests/runtime/Cargo.toml"), "--target-dir",
                               str(artifacts), "--target", target, *cargo_args]
                    built = subprocess.run(command, cwd=snapshot, stdout=output, stderr=subprocess.STDOUT)
                    if built.returncode:
                        raise RuntimeError(f"counterexample did not compile: {log}\n{log.read_text()[-4000:]}")
                    binary = artifacts / target / "release/clinkz-wot-td-admission-runtime"
                    command = [str(binary)] if target == native else [
                        qemu, "-M", "mps2-an386", "-display", "none", "-monitor", "none", "-serial", "none",
                        "-semihosting-config", "enable=on,target=native", "-kernel", str(binary)]
                    result = subprocess.run(command, cwd=snapshot, stdout=output,
                                            stderr=subprocess.STDOUT, timeout=300)
                observed = log.read_text()
                success = name == "baseline" or (arm_only and target == native)
                if success:
                    valid = result.returncode == 0 and "PASS: production TD runtime;" in observed
                elif target == native:
                    if name == "positive-uri-short-only":
                        valid = result.returncode == 1 and "independent positive URI/copy credit" in observed
                    else:
                        valid = result.returncode == -signal.SIGSEGV
                else:
                    marker = ("mandatory URI meaning" if name == "arm-missing-uri-charge" else
                              "independent positive URI/copy credit" if name == "positive-uri-short-only" else
                              "FAIL: forbidden TD access")
                    valid = result.returncode == 1 and marker in observed
                if name == "compensated-repair-copy-charge":
                    valid = valid and "two-byte copy predebit: action=padding" in observed
                if not valid:
                    raise RuntimeError(f"unexpected {name}/{target} exit={result.returncode}: {log}\n{observed[-4000:]}")
                print(f"{name}: {target} exit={result.returncode} expected={'PASS' if success else 'REJECT'}", flush=True)
    print("PASS: production acquisition/replacement and URI/copy predebit negative controls", flush=True)


if __name__ == "__main__":
    main()
