#!/usr/bin/env bash
set -euo pipefail
ulimit -c 0 # deliberate access-oracle controls must not leave core files

runtime_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$runtime_dir/../../.." && pwd)"
artifact_dir="$repo_dir/target/td-admission-runtime"
native_target="$(rustc -vV | sed -n 's/^host: //p')"
qemu="${QEMU_SYSTEM_ARM:-qemu-system-arm}"

rustc -vV
"$qemu" --version
for runtime_target in "$native_target" thumbv7em-none-eabihf; do
    cargo tree --locked --manifest-path "$runtime_dir/Cargo.toml" \
        --target "$runtime_target" --edges normal --format '{p} features=[{f}]' "$@"
    cargo build --locked --release --manifest-path "$runtime_dir/Cargo.toml" \
        --target-dir "$artifact_dir" --target "$runtime_target" "$@"
done

"$artifact_dir/$native_target/release/clinkz-wot-td-admission-runtime"
# A panic or ARM exception exits nonzero; a hung guest fails through timeout.
# This boots and executes the linked ELF rather than inspecting target metadata.
timeout 300 "$qemu" -M mps2-an386 -display none -monitor none -serial none \
    -semihosting-config enable=on,target=native \
    -kernel "$artifact_dir/thumbv7em-none-eabihf/release/clinkz-wot-td-admission-runtime"
