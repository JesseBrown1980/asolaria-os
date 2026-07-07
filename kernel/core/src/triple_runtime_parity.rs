//! Triple-runtime PID-fingerprint parity regression tests.
//!
//! These vectors mirror the workspace-level `kernel/tests/triple_runtime_parity.rs`
//! file so the parity check actually runs with the core crate test surface.

use crate::pid::{parse_pid, pid_fingerprint_sha16, validate_pid, ParsedPid};

const HISTORICAL_VECTORS: &[(&str, &str)] = &[
    ("OP-RAYSSA-PID-G0000-A00-W000", "bf5fa7a1a57f384b"),
    ("OPERATOR-PID-H1001-A00-W110", "760ba73b84f31861"),
    (
        "ASOLARIA-FEDERATION-REMAKE-1024-PID-2026-05-11",
        "e00b1a465d6dcb50",
    ),
];

const AETHER_MIGRATION_VECTOR: (&str, &str) =
    ("AETHER-CLAUDE-PID-G0049-A00-W001", "ac28f4ce43d17fea");

#[test]
fn historical_pids_match_cross_runtime_canonical_fingerprints() {
    for (pid, expected) in HISTORICAL_VECTORS {
        let rust_fp = pid_fingerprint_sha16(pid);
        assert_eq!(
            &rust_fp, expected,
            "PID {pid} sha16: rust got {rust_fp}, expected JS/Python parity {expected}"
        );
    }
}

#[test]
fn aether_migration_pid_matches_liris_verdict_fingerprint() {
    let (pid, expected) = AETHER_MIGRATION_VECTOR;
    let rust_fp = pid_fingerprint_sha16(pid);
    assert_eq!(&rust_fp, expected);
}

#[test]
fn historical_pids_all_validate_against_strict_role() {
    for (pid, _expected) in HISTORICAL_VECTORS {
        assert!(
            validate_pid(pid, true).is_ok(),
            "{pid} should strict-validate against KNOWN_ROLES"
        );
    }
}

#[test]
fn aether_migration_pid_parses_canonical_shape_but_role_not_yet_in_known_roles() {
    let (pid, _expected) = AETHER_MIGRATION_VECTOR;
    let parsed = parse_pid(pid).expect("aether migration PID should shape-parse");
    match parsed {
        ParsedPid::Canonical(parts) => {
            assert_eq!(parts.role, "AETHER-CLAUDE");
            assert_eq!(parts.region, b'G');
            assert_eq!(parts.host_code, "0049");
            assert_eq!(parts.activity, "00");
            assert_eq!(parts.wave, "001");
        }
        _ => panic!("expected Canonical form"),
    }
    assert!(
        validate_pid(pid, true).is_err(),
        "AETHER-CLAUDE role not yet canonical; tier-2 cosign needed to extend KNOWN_ROLES"
    );
}

#[test]
fn fingerprint_is_deterministic() {
    let pid = "OP-RAYSSA-PID-G0000-A00-W000";
    let fp1 = pid_fingerprint_sha16(pid);
    let fp2 = pid_fingerprint_sha16(pid);
    assert_eq!(fp1, fp2, "fingerprint must be byte-stable across calls");
}

#[test]
fn fingerprint_is_16_hex_chars() {
    let fp = pid_fingerprint_sha16("ASOLARIA-FEDERATION-REMAKE-1024-PID-2026-05-11");
    assert_eq!(fp.len(), 16);
    assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
}
