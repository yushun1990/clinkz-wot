#!/usr/bin/env bash
set -euo pipefail

script_dir="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
fixture_dir="$(CDPATH='' cd -- "$script_dir/.." && pwd)"
repo_dir="$(git -C "$script_dir" rev-parse --show-toplevel)"
manifest="$script_dir/Cargo.toml"
target="thumbv7em-none-eabihf"
binary_name="wp100-atomic-number-m4"

usage() {
    cat <<'EOF'
usage:
  run-board.sh prepare ARTIFACT_DIR BOARD_REVISION
  run-board.sh coverage ARTIFACT_DIR
  run-board.sh measure ARTIFACT_DIR
  run-board.sh validate ARTIFACT_DIR
  run-board.sh bundle ARTIFACT_DIR

Set WP100_PROBE to a probe-rs selector when more than one probe is attached.
Set WP100_COVERAGE_SECONDS to extend the default 10-second debugger timeout.
The coverage command requires the blue USER button to be held from reset until
the debugger reports WP100_SLOW_FALLBACK_HIT.
EOF
}

require_artifact_dir() {
    if [[ ! -d "$1" ]]; then
        echo "artifact directory does not exist: $1" >&2
        exit 1
    fi
}

require_probe_rs_version() {
    local artifact_dir="$1"
    local expected_version actual_version
    if [[ ! -f "$artifact_dir/probe-rs-version.txt" ]]; then
        echo "prepared probe-rs version is missing: $artifact_dir/probe-rs-version.txt" >&2
        exit 1
    fi
    expected_version="$(<"$artifact_dir/probe-rs-version.txt")"
    actual_version="$(probe-rs --version)"
    if [[ "$expected_version" != "probe-rs 0.32.0 (git commit: 48f5e4d)" ]]; then
        echo "coverage requires artifacts prepared with probe-rs 0.32.0 (48f5e4d); found: $expected_version" >&2
        exit 1
    fi
    if [[ "$actual_version" != "$expected_version" ]]; then
        echo "probe-rs version differs from the prepared artifact" >&2
        echo "prepared: $expected_version" >&2
        echo "current:  $actual_version" >&2
        exit 1
    fi
}

require_coverage_trace() {
    local coverage_file="$1"
    if [[ ! -f "$coverage_file" ]] \
        || ! grep -Fq 'WP100_SLOW_FALLBACK_HIT' "$coverage_file" \
        || ! grep -Eq 'parse_long_mantissa|dec2flt/slow.rs' "$coverage_file"; then
        echo "coverage trace does not contain a confirmed slow-fallback breakpoint hit" >&2
        exit 1
    fi
}

write_commands() {
    local artifact_dir="$1"
    local coverage_seconds="$2"
    local gdb_command="$3"
    {
        printf 'cargo build --release --locked --offline --target %q --manifest-path %q\n' \
            "$target" "$manifest"
        printf 'probe-rs download'
        printf ' %q' "${probe_args[@]}" --verify "$artifact_dir/firmware.elf"
        printf '\n'
        printf '# Hold the blue USER button before reset and through WP100_SLOW_FALLBACK_HIT.\n'
        printf 'probe-rs reset'
        printf ' %q' "${probe_args[@]}"
        printf '\n'
        printf 'timeout --foreground %q probe-rs gdb' "${coverage_seconds}s"
        printf ' %q' "${probe_args[@]}" --gdb "$gdb_command" "$artifact_dir/firmware.elf" -- \
            --batch -q \
            -ex 'set pagination off' \
            -ex 'set confirm off' \
            -ex 'hbreak library/core/src/num/dec2flt/slow.rs:39' \
            -ex continue \
            -ex 'printf "WP100_SLOW_FALLBACK_HIT\n"' \
            -ex frame \
            -ex bt \
            -ex quit
        printf '\n'
        printf '# Release the blue USER button before measurement.\n'
        printf 'probe-rs run'
        printf ' %q' "${probe_args[@]}" --verify \
            --target-output-file "semihosting:stdout=$artifact_dir/raw.jsonl" \
            "$artifact_dir/firmware.elf"
        printf '\n'
    } > "$artifact_dir/commands.txt"
}

record_disassembler_diagnostics() {
    local raw="$1"
    local destination="$2"
    {
        printf 'Disassembler diagnostic lines: '
        wc -l < "$raw"
        printf '\nFirst 20 diagnostics:\n'
        sed -n '1,20p' "$raw"
        printf '\nLast 20 diagnostics:\n'
        tail -20 "$raw"
    } > "$destination"
    rm "$raw"
}

probe_args=(--chip STM32F407VG --protocol swd --non-interactive)
if [[ -n "${WP100_PROBE:-}" ]]; then
    probe_args+=(--probe "$WP100_PROBE")
fi

prepare() {
    local artifact_dir="$1"
    local board_revision="$2"
    if [[ ! "$board_revision" =~ ^[A-Za-z0-9._+-]+$ ]]; then
        echo "BOARD_REVISION must use only letters, digits, dot, underscore, plus, or hyphen" >&2
        exit 1
    fi
    if [[ -e "$artifact_dir" && ! -d "$artifact_dir" ]]; then
        echo "artifact path exists and is not a directory: $artifact_dir" >&2
        exit 1
    fi
    if [[ -d "$artifact_dir" && -n "$(find "$artifact_dir" -mindepth 1 -print -quit)" ]]; then
        echo "prepare requires a new or empty artifact directory: $artifact_dir" >&2
        exit 1
    fi
    mkdir -p "$artifact_dir"
    artifact_dir="$(realpath "$artifact_dir")"

    cargo build --release --locked --offline --target "$target" --manifest-path "$manifest"
    local elf="$fixture_dir/target/$target/release/$binary_name"
    local map
    map="$(find "$fixture_dir/target/$target/release/build" -path "*/out/$binary_name.map" -printf '%T@ %p\n' | sort -n | tail -1 | cut -d' ' -f2-)"
    if [[ ! -f "$elf" || ! -f "$map" ]]; then
        echo "board ELF or linker map was not produced" >&2
        exit 1
    fi

    cp "$elf" "$artifact_dir/firmware.elf"
    cp "$map" "$artifact_dir/firmware.map"
    local llvm_objdump text_start text_size text_end disassembler_warnings
    llvm_objdump="$(find "$(rustc --print sysroot)" -type f -name llvm-objdump -print -quit)"
    rustc -Vv > "$artifact_dir/rustc-Vv.txt"
    cargo -V > "$artifact_dir/cargo-version.txt"
    probe-rs --version > "$artifact_dir/probe-rs-version.txt"
    cargo tree --locked --offline --target "$target" --manifest-path "$manifest" -e features \
        -p bounded-atomic-number-stm32f407-runner > "$artifact_dir/resolved-features.txt"
    readelf -hSWl "$artifact_dir/firmware.elf" > "$artifact_dir/elf-layout.txt"
    readelf -sW "$artifact_dir/firmware.elf" > "$artifact_dir/elf-symbols.txt"
    readelf --debug-dump=frames "$artifact_dir/firmware.elf" > "$artifact_dir/elf-frames.txt"
    if [[ -n "$llvm_objdump" ]]; then
        disassembler_warnings="$(mktemp)"
        "$llvm_objdump" --disassemble --demangle --line-numbers \
            "$artifact_dir/firmware.elf" > "$artifact_dir/firmware.disassembly.txt" \
            2> "$disassembler_warnings"
        record_disassembler_diagnostics \
            "$disassembler_warnings" "$artifact_dir/disassembler-warnings.txt"
        printf 'llvm-objdump: %s\n' "$llvm_objdump" > "$artifact_dir/disassembler.txt"
    elif command -v arm-none-eabi-objdump >/dev/null; then
        disassembler_warnings="$(mktemp)"
        arm-none-eabi-objdump --disassemble --demangle --line-numbers \
            "$artifact_dir/firmware.elf" > "$artifact_dir/firmware.disassembly.txt" \
            2> "$disassembler_warnings"
        record_disassembler_diagnostics \
            "$disassembler_warnings" "$artifact_dir/disassembler-warnings.txt"
        arm-none-eabi-objdump --version > "$artifact_dir/disassembler.txt"
    elif command -v gdb >/dev/null; then
        read -r text_start text_size < <(
            readelf -SW "$artifact_dir/firmware.elf" | awk '$3 == ".text" { print "0x" $5, "0x" $7 }'
        )
        text_end="$(printf '0x%x' "$((text_start + text_size))")"
        disassembler_warnings="$(mktemp)"
        gdb -batch \
            -ex 'set architecture arm' \
            -ex "file $artifact_dir/firmware.elf" \
            -ex "disassemble /r $text_start,$text_end" \
            > "$artifact_dir/firmware.disassembly.txt" \
            2> "$disassembler_warnings"
        record_disassembler_diagnostics \
            "$disassembler_warnings" "$artifact_dir/disassembler-warnings.txt"
        gdb --version > "$artifact_dir/disassembler.txt"
    else
        echo "an ARM-capable llvm-objdump, arm-none-eabi-objdump, or gdb is required" >&2
        exit 1
    fi
    git -C "$repo_dir" rev-parse HEAD > "$artifact_dir/git-head.txt"
    git -C "$repo_dir" status --short --branch > "$artifact_dir/git-status.txt"
    sha256sum "$fixture_dir/Cargo.lock" > "$artifact_dir/lockfile.sha256"
    sha256sum "$artifact_dir/firmware.elf" > "$artifact_dir/firmware.sha256"
    sha256sum "$artifact_dir/firmware.map" > "$artifact_dir/linker-map.sha256"
    sha256sum "$artifact_dir/firmware.disassembly.txt" > "$artifact_dir/disassembly.sha256"

    cat > "$artifact_dir/run-metadata.json" <<EOF
{
  "schema": "wp100-atomic-number-m4-build-metadata-v1",
  "workload_id": "WP100-ATOMIC-NUMBER-M4-v1",
  "board": "STM32F407G-DISC1",
  "mcu": "STM32F407VGT6",
  "board_revision": "$board_revision",
  "target": "$target",
  "chip_selector": "STM32F407VG",
  "protocol": "SWD",
  "code_placement": "flash",
  "data_and_measured_stack_placement": "SRAM",
  "main_stack_placement": "CCM",
  "release_profile": {
    "opt_level": 3,
    "debug": 2,
    "lto": false,
    "codegen_units": 1,
    "panic": "abort"
  },
  "admission_claim": false
}
EOF
    write_commands "$artifact_dir" 10 gdb
    echo "prepared $artifact_dir"
}

coverage() {
    local artifact_dir coverage_seconds gdb_command breakpoint_check coverage_status
    artifact_dir="$(realpath "$1")"
    require_artifact_dir "$artifact_dir"
    require_probe_rs_version "$artifact_dir"
    coverage_seconds="${WP100_COVERAGE_SECONDS:-10}"
    if [[ ! "$coverage_seconds" =~ ^[1-9][0-9]*$ ]]; then
        echo "WP100_COVERAGE_SECONDS must be a positive integer" >&2
        exit 1
    fi
    gdb_command="$(command -v gdb || true)"
    if [[ -z "$gdb_command" ]]; then
        echo "GDB is required for the probe-rs 0.32.0 slow-fallback coverage trace" >&2
        exit 1
    fi
    breakpoint_check="$(mktemp)"
    if ! "$gdb_command" -q -batch "$artifact_dir/firmware.elf" \
        -ex 'set architecture arm' \
        -ex 'break library/core/src/num/dec2flt/slow.rs:39' \
        -ex 'info breakpoints' > "$breakpoint_check" 2>&1 \
        || ! grep -Eq 'parse_long_mantissa|dec2flt/slow.rs' "$breakpoint_check"; then
        cat "$breakpoint_check" >&2
        rm "$breakpoint_check"
        echo "GDB could not resolve the slow-fallback source breakpoint in firmware.elf" >&2
        exit 1
    fi
    mv "$breakpoint_check" "$artifact_dir/slow-fallback-breakpoint.txt"
    probe-rs --version > "$artifact_dir/coverage-probe-rs-version.txt"
    "$gdb_command" --version > "$artifact_dir/gdb-version.txt"
    git -C "$repo_dir" rev-parse HEAD > "$artifact_dir/coverage-runner-git-head.txt"
    git -C "$repo_dir" status --short --branch > "$artifact_dir/coverage-runner-git-status.txt"
    write_commands "$artifact_dir" "$coverage_seconds" "$gdb_command"

    probe-rs download "${probe_args[@]}" --verify "$artifact_dir/firmware.elf"
    echo "Hold the blue USER button now and keep it held through WP100_SLOW_FALLBACK_HIT." >&2
    read -r -p "Press Enter when the USER button is held: "
    probe-rs reset "${probe_args[@]}"
    set +e
    timeout --foreground "${coverage_seconds}s" \
        probe-rs gdb "${probe_args[@]}" --gdb "$gdb_command" \
        "$artifact_dir/firmware.elf" -- \
        --batch -q \
        -ex 'set pagination off' \
        -ex 'set confirm off' \
        -ex 'hbreak library/core/src/num/dec2flt/slow.rs:39' \
        -ex continue \
        -ex 'printf "WP100_SLOW_FALLBACK_HIT\n"' \
        -ex frame \
        -ex bt \
        -ex quit \
        2>&1 | tee "$artifact_dir/slow-fallback-coverage.txt"
    coverage_status=${PIPESTATUS[0]}
    set -e
    if [[ $coverage_status -ne 0 ]]; then
        echo "coverage debugger trace failed or timed out; output was preserved" >&2
        return 2
    fi
    require_coverage_trace "$artifact_dir/slow-fallback-coverage.txt"
    sha256sum "$artifact_dir/slow-fallback-coverage.txt" > "$artifact_dir/coverage.sha256"
}

measure() {
    local artifact_dir
    artifact_dir="$(realpath "$1")"
    require_artifact_dir "$artifact_dir"
    require_probe_rs_version "$artifact_dir"
    require_coverage_trace "$artifact_dir/slow-fallback-coverage.txt"
    echo "Release the blue USER button before measurement reset." >&2
    probe-rs list > "$artifact_dir/probe-list.txt"
    set +e
    probe-rs run "${probe_args[@]}" --verify \
        --target-output-file "semihosting:stdout=$artifact_dir/raw.jsonl" \
        "$artifact_dir/firmware.elf" > "$artifact_dir/probe-rs-run.log" 2>&1
    local probe_status=$?
    set -e
    printf '%s\n' "$probe_status" > "$artifact_dir/probe-rs-exit-status.txt"
    set +e
    python3 "$script_dir/validate.py" "$artifact_dir/raw.jsonl" \
        --coverage "$artifact_dir/slow-fallback-coverage.txt" \
        --summary "$artifact_dir/summary.json" | tee "$artifact_dir/validation.log"
    local validation_status=${PIPESTATUS[0]}
    set -e
    sha256sum "$artifact_dir/raw.jsonl" > "$artifact_dir/raw.sha256"
    sha256sum "$artifact_dir/summary.json" > "$artifact_dir/summary.sha256"
    if [[ $probe_status -ne 0 || $validation_status -ne 0 ]]; then
        echo "measurement was preserved but did not validate as a passing candidate" >&2
        return 2
    fi
}

validate() {
    local artifact_dir
    artifact_dir="$(realpath "$1")"
    require_artifact_dir "$artifact_dir"
    python3 "$script_dir/validate.py" "$artifact_dir/raw.jsonl" \
        --coverage "$artifact_dir/slow-fallback-coverage.txt" \
        --summary "$artifact_dir/summary.json"
}

bundle() {
    local artifact_dir parent name archive
    artifact_dir="$(realpath "$1")"
    require_artifact_dir "$artifact_dir"
    parent="$(dirname "$artifact_dir")"
    name="$(basename "$artifact_dir")"
    archive="$parent/$name.tar.gz"
    tar -czf "$archive" -C "$parent" "$name"
    sha256sum "$archive" > "$archive.sha256"
    echo "$archive"
}

if [[ $# -lt 2 ]]; then
    usage >&2
    exit 1
fi

command="$1"
shift
case "$command" in
    prepare)
        [[ $# -eq 2 ]] || { usage >&2; exit 1; }
        prepare "$1" "$2"
        ;;
    coverage)
        [[ $# -eq 1 ]] || { usage >&2; exit 1; }
        coverage "$1"
        ;;
    measure)
        [[ $# -eq 1 ]] || { usage >&2; exit 1; }
        measure "$1"
        ;;
    validate)
        [[ $# -eq 1 ]] || { usage >&2; exit 1; }
        validate "$1"
        ;;
    bundle)
        [[ $# -eq 1 ]] || { usage >&2; exit 1; }
        bundle "$1"
        ;;
    *)
        usage >&2
        exit 1
        ;;
esac
