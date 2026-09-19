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
Set WP100_COVERAGE_SECONDS to extend the default 10-second profile.
The coverage command requires the blue USER button to be held from reset until
the profiler has entered the repeated halfway-case projection loop.
EOF
}

require_artifact_dir() {
    if [[ ! -d "$1" ]]; then
        echo "artifact directory does not exist: $1" >&2
        exit 1
    fi
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
    cat > "$artifact_dir/commands.txt" <<EOF
cargo build --release --locked --offline --target $target --manifest-path $manifest
probe-rs profile --chip STM32F407VG --protocol swd --duration 10 --flash --reset firmware.elf flat --line-info --limit 200 naive
probe-rs run --chip STM32F407VG --protocol swd --verify --target-output-file semihosting:stdout=raw.jsonl firmware.elf
EOF
    echo "prepared $artifact_dir"
}

coverage() {
    local artifact_dir coverage_seconds
    artifact_dir="$(realpath "$1")"
    require_artifact_dir "$artifact_dir"
    coverage_seconds="${WP100_COVERAGE_SECONDS:-10}"
    if [[ ! "$coverage_seconds" =~ ^[1-9][0-9]*$ ]]; then
        echo "WP100_COVERAGE_SECONDS must be a positive integer" >&2
        exit 1
    fi
    echo "Hold the blue USER button now and keep it held until profiling starts." >&2
    probe-rs profile "${probe_args[@]}" --duration "$coverage_seconds" --flash --reset \
        "$artifact_dir/firmware.elf" flat --line-info --limit 200 naive \
        2>&1 | tee "$artifact_dir/slow-fallback-coverage.txt"
    if ! grep -Eq 'parse_long_mantissa|dec2flt/slow.rs' "$artifact_dir/slow-fallback-coverage.txt"; then
        echo "coverage did not observe the slow fallback; repeat with WP100_COVERAGE_SECONDS increased" >&2
        exit 2
    fi
    sha256sum "$artifact_dir/slow-fallback-coverage.txt" > "$artifact_dir/coverage.sha256"
}

measure() {
    local artifact_dir
    artifact_dir="$(realpath "$1")"
    require_artifact_dir "$artifact_dir"
    if [[ ! -f "$artifact_dir/slow-fallback-coverage.txt" ]]; then
        echo "run the same-ELF coverage step first" >&2
        exit 1
    fi
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
