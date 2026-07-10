#!/usr/bin/env bash
set -uo pipefail

SKIP_QEMU=0
QEMU_TIMEOUT_SECONDS="${QEMU_TIMEOUT_SECONDS:-15}"

usage() {
    echo "usage: $0 [--skip-qemu] [--qemu-timeout seconds]"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-qemu)
            SKIP_QEMU=1
            shift
            ;;
        --qemu-timeout)
            if [[ $# -lt 2 || ! "$2" =~ ^[0-9]+$ || "$2" -lt 1 ]]; then
                echo "[fail] --qemu-timeout requires a positive integer"
                usage
                exit 1
            fi
            QEMU_TIMEOUT_SECONDS="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "[fail] unknown argument: $1"
            usage
            exit 1
            ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KERNEL_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_DIR="${KERNEL_DIR}/dist"
TARGET_TRIPLE="x86_64-unknown-uefi"
TARGET_ARTIFACT="${KERNEL_DIR}/target/${TARGET_TRIPLE}/release/asolaria-os.efi"
DIST_ARTIFACT="${DIST_DIR}/asolaria-os-x86_64.efi"
QEMU_FAT_ROOT="${DIST_DIR}/qemu-fat-root"
FAILURES=0

test_tool() {
    command -v "$1" >/dev/null 2>&1
}

honest_fail() {
    echo "[honest_fail] $1"
    exit 2
}

assert_not_symlink() {
    local path="$1"
    local label="$2"
    if [[ -L "${path}" ]]; then
        honest_fail "${label} is a symlink, refusing local artifact write: ${path}"
    fi
}

add_failure() {
    local label="$1"
    local exit_code="$2"
    echo "[fail] ${label} exit=${exit_code}"
    FAILURES=$((FAILURES + 1))
}

run_step() {
    local label="$1"
    shift

    echo
    echo "[run] ${label}"
    "$@"
    local exit_code=$?
    if [[ "${exit_code}" -eq 0 ]]; then
        echo "[ok] ${label}"
        return 0
    fi

    add_failure "${label}" "${exit_code}"
    return 1
}

print_hash() {
    local path="$1"
    if [[ ! -f "${path}" ]]; then
        add_failure "artifact missing: ${path}" 1
        return
    fi

    local hash
    if test_tool sha256sum; then
        hash="$(sha256sum "${path}" | awk '{print $1}')"
    elif test_tool shasum; then
        hash="$(shasum -a 256 "${path}" | awk '{print $1}')"
    else
        honest_fail "neither sha256sum nor shasum is available for artifact hashing"
    fi

    echo "[artifact] ${path}"
    echo "[sha256] ${hash}"
    echo "[sha16]  ${hash:0:16}"
}

verify_artifact_markers() {
    local path="$1"
    local required_markers=(
        "seat=liris"
        "device_identity=runtime-pci-60d"
        "BOOTPID|device_pid="
        "BOOTTIME|utc="
        "BOOTPROJ|boot_pid="
        "BOOTDRIVER|driver=intel-rst-vmd"
    )
    local forbidden_markers=(
        "ACER-CLAUDE-FABLE5"
        "|colony=acer|"
    )

    local marker
    for marker in "${required_markers[@]}"; do
        if grep -aFq -- "${marker}" "${path}"; then
            echo "[ok] artifact marker present: ${marker}"
        else
            add_failure "artifact marker missing: ${marker}" 1
        fi
    done

    for marker in "${forbidden_markers[@]}"; do
        if grep -aFq -- "${marker}" "${path}"; then
            add_failure "forbidden cross-seat artifact marker present: ${marker}" 1
        else
            echo "[ok] forbidden artifact marker absent: ${marker}"
        fi
    done
}

find_ovmf_code() {
    if [[ -n "${OVMF_CODE:-}" ]]; then
        if [[ -f "${OVMF_CODE}" ]]; then
            echo "${OVMF_CODE}"
            return 0
        fi

        echo "[honest_skip] QEMU smoke skipped: OVMF_CODE is set but does not point to a file: ${OVMF_CODE}" >&2
        return 1
    fi

    local candidates=(
        "/usr/share/OVMF/OVMF_CODE.fd"
        "/usr/share/OVMF/OVMF_CODE_4M.fd"
        "/usr/share/ovmf/OVMF.fd"
        "/usr/share/qemu/OVMF.fd"
        "/usr/share/edk2/x64/OVMF_CODE.fd"
        "/usr/share/edk2/ovmf/OVMF_CODE.fd"
    )

    local candidate
    for candidate in "${candidates[@]}"; do
        if [[ -f "${candidate}" ]]; then
            echo "${candidate}"
            return 0
        fi
    done

    echo "[honest_skip] QEMU smoke skipped: qemu-system-x86_64 is installed, but OVMF_CODE was not found" >&2
    return 1
}

run_qemu_smoke() {
    if [[ "${SKIP_QEMU}" -eq 1 ]]; then
        echo "[skip] QEMU smoke skipped by --skip-qemu"
        return
    fi

    if ! test_tool qemu-system-x86_64; then
        echo "[honest_skip] QEMU smoke skipped: qemu-system-x86_64 not found"
        return
    fi

    local ovmf_code
    if ! ovmf_code="$(find_ovmf_code)"; then
        return
    fi

    if [[ ! -f "${DIST_ARTIFACT}" ]]; then
        echo "[honest_skip] QEMU smoke skipped: artifact missing at ${DIST_ARTIFACT}"
        return
    fi

    if ! test_tool timeout; then
        echo "[honest_skip] QEMU smoke skipped: timeout command not found"
        return
    fi

    assert_not_symlink "${DIST_DIR}" "dist dir"
    assert_not_symlink "${QEMU_FAT_ROOT}" "qemu fat root"
    mkdir -p "${QEMU_FAT_ROOT}/EFI/BOOT"
    assert_not_symlink "${QEMU_FAT_ROOT}" "qemu fat root"
    cp "${DIST_ARTIFACT}" "${QEMU_FAT_ROOT}/EFI/BOOT/BOOTX64.EFI"

    echo
    echo "[run] qemu-system-x86_64 smoke timeout=${QEMU_TIMEOUT_SECONDS}s"
    # OVMF may touch the boot volume during startup. This is a temporary local FAT directory under kernel/dist.
    timeout "${QEMU_TIMEOUT_SECONDS}s" \
        qemu-system-x86_64 \
            -machine q35 \
            -m 512 \
            -display none \
            -serial none \
            -monitor none \
            -no-reboot \
            -drive if=pflash,format=raw,readonly=on,file="${ovmf_code}" \
            -drive format=raw,file=fat:rw:"${QEMU_FAT_ROOT}"
    local exit_code=$?

    case "${exit_code}" in
        0)
            echo "[ok] QEMU liveness smoke exited cleanly; this is not a boot-banner or metal proof"
            ;;
        124)
            echo "[ok] QEMU liveness smoke stayed up until timeout; this is not a boot-banner or metal proof"
            ;;
        *)
            add_failure "QEMU smoke" "${exit_code}"
            ;;
    esac
}

echo "[suite] Asolaria kernel local build/unit harness (bash)"
echo "[safety] local-only build artifacts; no USB/ESP/BCD writes; no diskpart, bcdedit, mountvol, Format-Volume, mkfs, or dd calls"
echo "[root] ${KERNEL_DIR}"

test_tool cargo || honest_fail "cargo not found; install rustup/cargo. The kernel workspace selects Rust 1.81 from rust-toolchain.toml."
test_tool rustc || honest_fail "rustc not found; install Rust 1.81 through rustup."
test_tool rustfmt || honest_fail "rustfmt not found; install the Rust 1.81 rustfmt component."

cd "${KERNEL_DIR}" || exit 1

RUSTC_VERSION="$(rustc --version 2>&1)"
if [[ $? -ne 0 ]]; then
    honest_fail "rustc --version failed"
fi
echo "[tool] ${RUSTC_VERSION}"
if [[ ! "${RUSTC_VERSION}" =~ ^rustc\ 1\.81\. ]]; then
    honest_fail "kernel rust-toolchain.toml must resolve to Rust 1.81; observed '${RUSTC_VERSION}'"
fi

CARGO_VERSION="$(cargo --version 2>&1)"
if [[ $? -ne 0 ]]; then
    honest_fail "cargo --version failed"
fi
echo "[tool] ${CARGO_VERSION}"

if test_tool rustup; then
    INSTALLED_TARGETS="$(rustup target list --toolchain 1.81 --installed 2>/dev/null)"
    if [[ $? -eq 0 && "${INSTALLED_TARGETS}" != *"${TARGET_TRIPLE}"* ]]; then
        honest_fail "rustup target '${TARGET_TRIPLE}' is not installed for toolchain 1.81. Run: rustup target add ${TARGET_TRIPLE} --toolchain 1.81"
    fi
fi

run_step "cargo fmt --all --check" cargo fmt --all --check
run_step "cargo check -p asolaria-kernel-core --all-targets" cargo check -p asolaria-kernel-core --all-targets
run_step "cargo test -p asolaria-kernel-core --all-targets -- --test-threads=1" cargo test -p asolaria-kernel-core --all-targets -- --test-threads=1
if run_step "cargo build --release --target ${TARGET_TRIPLE} --bin asolaria-os" cargo build --release --target "${TARGET_TRIPLE}" --bin asolaria-os; then
    if [[ -f "${TARGET_ARTIFACT}" ]]; then
        verify_artifact_markers "${TARGET_ARTIFACT}"
        assert_not_symlink "${DIST_DIR}" "dist dir"
        mkdir -p "${DIST_DIR}"
        assert_not_symlink "${DIST_DIR}" "dist dir"
        if cp "${TARGET_ARTIFACT}" "${DIST_ARTIFACT}"; then
            print_hash "${TARGET_ARTIFACT}"
            print_hash "${DIST_ARTIFACT}"
        else
            add_failure "copy artifact to ${DIST_ARTIFACT}" 1
            print_hash "${TARGET_ARTIFACT}"
            echo "[honest_skip] dist artifact hash skipped because artifact copy failed"
        fi
    else
        add_failure "artifact missing: ${TARGET_ARTIFACT}" 1
    fi
    run_qemu_smoke
else
    echo "[honest_skip] artifact hash skipped: UEFI build failed, avoiding stale artifact claim"
    echo "[honest_skip] QEMU smoke skipped: UEFI build failed"
fi

if [[ "${FAILURES}" -eq 0 ]]; then
    echo
    echo "[done] local build/unit suite passed; skipped or passing QEMU smoke is not a system-test proof"
    exit 0
fi

echo
echo "[done] suite failed failures=${FAILURES}"
exit 3
