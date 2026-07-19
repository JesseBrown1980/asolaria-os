//! MTP watcher trio (MTP1 / MTP2 / MTP3) · Phase-3 consistency gate.
//!
//! The three named consistency observers from the Q-PRISM watcher gate, ported into
//! the kernel as a pure `no_std` verification contract. They implement the
//! reconstruct-then-reproject law: a black-side projection produces a signature; a
//! white-side candidate is re-projected to its own signature; the gate emits
//! `Verified` (AuthorityState::Measured) ONLY if all three watchers independently
//! agree AND the joint CRT capacity is sufficient. Any disagreement → `Held`.
//!
//! - **MTP1** — pixel-slice consistency (the raw slice bytes match).
//! - **MTP2** — frequency-shell consistency (the spectral shell descriptor matches).
//! - **MTP3** — cylinder-residue consistency (every CRT residue matches) AND the
//!   selected pairwise-coprime cylinders jointly cover the value range
//!   (`prod(p_i) >= 2^range_bits`), else the shadows are individually non-injective
//!   and the gate must HOLD rather than guess (Path-2 sufficiency law).
//!
//! This module launches no model and asserts no authority by placement: it only
//! recomputes truth and compares. Pairs 1:1 with `triple_runtime_parity` (three
//! runtimes agree) and the N-Nest invariant (child.reported == watcher.recomputed).

use alloc::vec::Vec;
use sha2::{Digest, Sha256};

use crate::reflection_room::AuthorityState;

/// A projected signature: the facets each MTP watcher checks. Cheap to clone in tests;
/// in the live path these ride as HBP rows, never as raw payload bodies.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Signature {
    /// SHA-256 identity of the reconstructed object.
    pub sha: [u8; 32],
    /// MTP1 facet — the pixel slice bytes.
    pub pixels: Vec<u8>,
    /// MTP2 facet — frequency-shell descriptor (bounded spectral bins).
    pub shells: Vec<u32>,
    /// MTP3 facet — pairwise-coprime `(prime, residue)` cylinders.
    pub cylinders: Vec<(u64, u64)>,
}

impl Signature {
    /// Recompute the SHA facet from the pixel bytes (used to build a black signature).
    pub fn seal_sha(&mut self) {
        let mut h = Sha256::new();
        h.update(&self.pixels);
        self.sha.copy_from_slice(&h.finalize());
    }
}

/// Why the gate refused to emit a verified clone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MtpHold {
    /// MTP1 disagreed: reconstructed pixels differ from the black slice.
    PixelMismatch,
    /// MTP2 disagreed: frequency shells differ.
    ShellMismatch,
    /// MTP3 disagreed: at least one cylinder residue differs.
    CylinderMismatch,
    /// MTP3 capacity: selected cylinders do not jointly cover the range (non-injective).
    InsufficientJointCapacity,
    /// Whole-object identity differs even though facets were checked.
    ShaMismatch,
}

/// Verdict of the trio. `Verified` carries `AuthorityState::Measured`; a hold is never
/// silently upgraded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MtpVerdict {
    Verified,
    Held(MtpHold),
}

impl MtpVerdict {
    /// Map to the kernel-wide authority vocabulary. A verified trio is `Measured`
    /// evidence, never `Canon`; a hold is `Held`.
    pub fn authority(self) -> AuthorityState {
        match self {
            MtpVerdict::Verified => AuthorityState::Measured,
            MtpVerdict::Held(_) => AuthorityState::Held,
        }
    }
    pub fn is_verified(self) -> bool {
        matches!(self, MtpVerdict::Verified)
    }
}

/// A committed receipt: the three watcher bits plus the sealed authority. Content-free
/// by construction — it holds booleans and a verdict, never the object bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MtpReceipt {
    pub mtp1_pixels_ok: bool,
    pub mtp2_shells_ok: bool,
    pub mtp3_cylinders_ok: bool,
    pub joint_capacity_ok: bool,
    pub verdict: MtpVerdict,
}

// ---- the three independent watchers ----

/// MTP1 — pixel-slice consistency.
pub fn mtp1_pixel(black: &Signature, white: &Signature) -> bool {
    black.pixels == white.pixels
}

/// MTP2 — frequency-shell consistency.
pub fn mtp2_shell(black: &Signature, white: &Signature) -> bool {
    black.shells == white.shells
}

/// MTP3 residue check — every reconstructed residue matches the black cylinder set.
pub fn mtp3_cylinder(black: &Signature, white: &Signature) -> bool {
    black.cylinders == white.cylinders
}

/// MTP3 capacity — the selected pairwise-coprime cylinders jointly cover `2^range_bits`.
/// Returns false (→ hold) if the product of primes is below the range, i.e. the shadows
/// are individually non-injective and jointly still ambiguous. Overflow-safe: it works in
/// log2 space so 128-bit products never wrap.
pub fn mtp3_joint_capacity(cylinders: &[(u64, u64)], range_bits: u32) -> bool {
    let mut bits: u32 = 0;
    for &(p, _r) in cylinders {
        if p <= 1 {
            continue;
        }
        // floor(log2(p)) + 1 == number of bits; sum is a safe lower bound on log2(prod)
        bits = bits.saturating_add(64 - (p - 1).leading_zeros());
        if bits >= range_bits {
            return true;
        }
    }
    bits >= range_bits
}

/// The watcher gate: emit `Verified` only if the joint capacity is sufficient AND all
/// three watchers agree AND the whole-object SHA matches. Order matters — capacity is
/// checked first so an under-determined reconstruction is HELD, not compared.
pub fn watcher_gate(black: &Signature, white: &Signature, range_bits: u32) -> MtpReceipt {
    let cap_ok = mtp3_joint_capacity(&black.cylinders, range_bits);
    let p_ok = mtp1_pixel(black, white);
    let s_ok = mtp2_shell(black, white);
    let c_ok = mtp3_cylinder(black, white);

    let verdict = if !cap_ok {
        MtpVerdict::Held(MtpHold::InsufficientJointCapacity)
    } else if !p_ok {
        MtpVerdict::Held(MtpHold::PixelMismatch)
    } else if !s_ok {
        MtpVerdict::Held(MtpHold::ShellMismatch)
    } else if !c_ok {
        MtpVerdict::Held(MtpHold::CylinderMismatch)
    } else if black.sha != white.sha {
        MtpVerdict::Held(MtpHold::ShaMismatch)
    } else {
        MtpVerdict::Verified
    };

    MtpReceipt {
        mtp1_pixels_ok: p_ok,
        mtp2_shells_ok: s_ok,
        mtp3_cylinders_ok: c_ok,
        joint_capacity_ok: cap_ok,
        verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn black() -> Signature {
        let mut s = Signature {
            sha: [0u8; 32],
            pixels: vec![10, 20, 30, 40, 50, 60],
            shells: vec![100, 200, 300],
            // two ~25-bit coprime cylinders -> ~50 bits, covers a 48-bit slice
            cylinders: vec![(33554393, 7), (33554467, 11)],
        };
        s.seal_sha();
        s
    }

    #[test]
    fn identical_reprojection_is_verified_and_measured() {
        let b = black();
        let w = b.clone();
        let r = watcher_gate(&b, &w, 48);
        assert!(r.verdict.is_verified());
        assert_eq!(r.verdict.authority(), AuthorityState::Measured);
        assert!(r.mtp1_pixels_ok && r.mtp2_shells_ok && r.mtp3_cylinders_ok && r.joint_capacity_ok);
    }

    #[test]
    fn one_flipped_pixel_holds_on_mtp1() {
        let b = black();
        let mut w = b.clone();
        w.pixels[2] ^= 1;
        w.seal_sha();
        let r = watcher_gate(&b, &w, 48);
        assert_eq!(r.verdict, MtpVerdict::Held(MtpHold::PixelMismatch));
    }

    #[test]
    fn shell_drift_holds_on_mtp2() {
        let b = black();
        let mut w = b.clone();
        w.shells[1] += 1;
        let r = watcher_gate(&b, &w, 48);
        assert_eq!(r.verdict, MtpVerdict::Held(MtpHold::ShellMismatch));
    }

    #[test]
    fn one_residue_off_holds_on_mtp3() {
        let b = black();
        let mut w = b.clone();
        w.cylinders[0].1 += 1;
        let r = watcher_gate(&b, &w, 48);
        assert_eq!(r.verdict, MtpVerdict::Held(MtpHold::CylinderMismatch));
    }

    #[test]
    fn insufficient_cylinders_hold_before_any_comparison() {
        let mut b = black();
        b.cylinders = vec![(33554393, 7)]; // one ~25-bit cylinder cannot cover 48 bits
        let w = b.clone();
        let r = watcher_gate(&b, &w, 48);
        assert_eq!(r.verdict, MtpVerdict::Held(MtpHold::InsufficientJointCapacity));
        assert!(!r.joint_capacity_ok);
    }

    #[test]
    fn capacity_law_matches_path2_threshold() {
        // two 25-bit cylinders = ~50 bits: sufficient for 48, insufficient for 64
        let cyl = vec![(33554393u64, 0u64), (33554467u64, 0u64)];
        assert!(mtp3_joint_capacity(&cyl, 48));
        assert!(!mtp3_joint_capacity(&cyl, 64));
    }
}
