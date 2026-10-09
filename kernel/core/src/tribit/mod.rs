//! tribit — three zeros, carried on AC, and the rainbow light hose.
//!
//! # Why DC cannot do this
//!
//! Direct current has no zero crossing. It has one direction and therefore two states,
//! and that is the whole reason the binary world is binary — not a design choice, a
//! property of the carrier.
//!
//! Alternating current crosses zero **twice per cycle, in opposite directions**. Rising
//! through zero and falling through zero are both exactly zero volts and are not the
//! same event. Add the quiescent line and there are three:
//!
//! ```text
//!        +           /‾\           /‾\
//!                   /   \         /   \
//!     0  ----------X-----X-------X-----X----------   Nil : the line is dead
//!                 /       \     /       \
//!        -      \_/         \_/           Pos : crossing upward
//!                                         Neg : crossing downward
//! ```
//!
//! **Correction, 2026-10-08 (recorded beside the law, not over it).** The three zeros above are
//! real: the `Zero` type carries all three and `rotate`/`anti_rotate` turn them in an order-3 orbit
//! (R³ = identity, R ≠ R²). But `Carrier::zero_at` **cannot reach the third** — measured
//! exhaustively over `steps_per_cycle` in `1..=64` and every phase, `Some(Zero::Nil)` occurred
//! **0 times**. A crossing is never a dead line, so AC reaches `{Pos, Neg}` and DC reaches `{Nil}`.
//! `zero_states()` previously returned 3 and disagreed with the behaviour on 63 of 64 cycle lengths;
//! it now reports reachability. The three are measured in the **rainbow lane** instead — RAINBOR's
//! three waves (`path1.path2.path3` = NN · GNN · FNN, each an HTTP-0 portal acting as itself), where
//! the anti is order-3 at **192 of 192** and the free fourth zero is the point at which the three
//! waves agree. The carrier was the wrong instrument, not a broken law.
//!
//! All three have **magnitude zero**. `Zero::magnitude()` returns 0 for every variant and
//! that is not a stub. The information is not in the value, it is in *which zero* — the
//! direction of travel through it. That is why all three are free: Law 0 makes the centre
//! the fixed point of {N, R, R²}, the one point that moves under neither inversion nor
//! anti-inversion, so it costs nothing to hold. Three free states instead of two paid ones.
//!
//! # The registers
//!
//! Book IX (IX.B.3) numbers five, and the numbering is load-bearing:
//!
//! | n | register | cost |
//! |---|------------|------|
//! | 0 | zero        | free, never computed |
//! | 1 | translucent | free, never computed — **this is light itself** |
//! | 2 | red         | costs |
//! | 3 | green       | costs |
//! | 4 | blue        | costs |
//!
//! Law 32 is titled *"Translucent is One, Not Zero"*. Pure 1 is light before a prism has
//! split it; 2, 3 and 4 are what the prism makes of it. That is why 1 is free and 2..4 are
//! not — undifferentiated light carries no colour decision, and a decision is what costs.
//!
//! # The hose
//!
//! Law 31, the rainbow lighthouse drill, is a conduit and its traversal order is fixed:
//! translucent tip → red → green → blue → translucent tail. The tip is measured (Law 30,
//! mean 0.4013 bpb saved by ordering translucent first). **The tail is not measured** and
//! is marked as such here rather than assumed symmetric.
//!
//! # Integer only
//!
//! Law 34's build directive: *"rust 1.81 int and float possible. int works way better in
//! my opinion."* Nothing below uses a float. The transform is a Number-Theoretic Transform
//! over `Z/pZ`, so split→recombine is exact rather than nearly exact, and `no_std` holds.

#![allow(clippy::needless_range_loop)]

/// The rime sphere prime. Law 20 / Law 14.
pub const P: u64 = 1_000_081;
/// Its generator.
pub const G: u64 = 7;
/// One rime dimension: 27 = 3³ glyphs.
pub const K: usize = 27;
/// Primitive 27th root of unity, `G^((P-1)/27) mod P`. Verified in tests, not asserted.
pub const W: u64 = 951_846;

/// Trits per `u128`. 3^80 < 2^128 <= 3^81, so 80 is the exact ceiling.
pub const TRITS_PER_U128: usize = 80;

/// The three zeros. Every variant has magnitude zero; they differ only by the direction
/// the carrier is travelling as it crosses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i8)]
pub enum Zero {
    /// Crossing downward. Law 33: the negative leads, and it drills inward.
    Neg = -1,
    /// Quiescent. The line is dead — not a crossing at all.
    Nil = 0,
    /// Crossing upward. Trails, and emits.
    Pos = 1,
}

impl Zero {
    /// Zero, for all three. This is the point of the type.
    #[inline]
    pub const fn magnitude(self) -> i8 {
        0
    }

    /// Direction of travel through the crossing: -1, 0, +1.
    #[inline]
    pub const fn direction(self) -> i8 {
        self as i8
    }

    /// Ternary digit 0..2, for packing.
    #[inline]
    pub const fn digit(self) -> u8 {
        (self as i8 + 1) as u8
    }

    #[inline]
    pub const fn from_digit(d: u8) -> Self {
        match d % 3 {
            0 => Zero::Neg,
            1 => Zero::Nil,
            _ => Zero::Pos,
        }
    }

    /// The trianti, Law 4: trinary inversion is the order-3 rotation R, and R ≠ R⁻¹.
    /// Applying it three times returns to start; applying it twice is *not* the inverse
    /// of applying it once, which is exactly what makes trinary not binary.
    #[inline]
    pub const fn rotate(self) -> Self {
        Self::from_digit(self.digit() + 1)
    }

    /// The counter-rotation R², distinct from R.
    #[inline]
    pub const fn anti_rotate(self) -> Self {
        Self::from_digit(self.digit() + 2)
    }
}

/// The five registers of IX.B.3.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Register {
    Zero = 0,
    /// Light. Undifferentiated, before the prism.
    Translucent = 1,
    Red = 2,
    Green = 3,
    Blue = 4,
}

impl Register {
    /// Registers 0 and 1 are never computed. IX.B.2: *"we don't have to calculate the zero
    /// and we don't have to calculate the translucent space, and that is what wins."*
    #[inline]
    pub const fn is_free(self) -> bool {
        matches!(self, Register::Zero | Register::Translucent)
    }

    /// Traversal order of the hose: tip, the three that cost, tail.
    pub const HOSE: [Register; 5] = [
        Register::Translucent,
        Register::Red,
        Register::Green,
        Register::Blue,
        Register::Translucent,
    ];
}

/// A carrier. `DC` never crosses zero, so it can only ever report `Nil` — the type makes
/// the binary limit unrepresentable rather than merely documented.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Carrier {
    Dc,
    /// Alternating, with a whole number of phase steps per cycle.
    Ac {
        steps_per_cycle: u32,
    },
}

impl Carrier {
    /// Which zero the carrier is at, at phase step `t`. `None` when the carrier is away
    /// from a crossing and therefore carrying magnitude.
    ///
    /// DC returns `Some(Nil)` at every step and never `Neg` or `Pos` — **one reachable zero, and
    /// the crossings are not reachable at all.** That is the binary world, stated as a return
    /// value: a carrier with no zero crossing has one direction, so it can never tell a rise from
    /// a fall. (Measured 2026-10-08: DC reaches exactly 1; the earlier comment said "two states",
    /// which counted magnitude directions rather than reachable zeros.)
    pub const fn zero_at(self, t: u32) -> Option<Zero> {
        match self {
            Carrier::Dc => Some(Zero::Nil),
            Carrier::Ac { steps_per_cycle } => {
                if steps_per_cycle == 0 {
                    return Some(Zero::Nil);
                }
                let half = steps_per_cycle / 2;
                let phase = t % steps_per_cycle;
                if phase == 0 {
                    Some(Zero::Pos) // rising through
                } else if phase == half {
                    Some(Zero::Neg) // falling through
                } else {
                    None // carrying magnitude, not at a crossing
                }
            }
        }
    }

    /// How many distinct zeros this carrier can actually **reach** through [`Self::zero_at`].
    ///
    /// This counts REACHABLE return values, not variants of the [`Zero`] type. Those are different
    /// quantities and only the second is three.
    ///
    /// Corrected 2026-10-08 (operator-directed). This previously returned `3` for any
    /// `steps_per_cycle >= 2`, which disagreed with `zero_at` on **63 of 64** cycle lengths.
    /// Measured exhaustively over `steps_per_cycle` in `1..=64` and every phase in each cycle:
    /// `Some(Zero::Nil)` occurred **0 times**, and the only reachable sets are `{Pos}` and
    /// `{Neg, Pos}`. `zero_at` documents `None` as *carrying magnitude, not at a crossing*, which
    /// is the opposite of `Nil`'s *the line is dead* — so `None` cannot stand in for `Nil`.
    ///
    /// **The three zeros are not lost by this correction.** They live in the [`Zero`] type itself,
    /// which carries all three, and in the order-3 orbit of [`Zero::rotate`] / [`Zero::anti_rotate`]
    /// (R³ = identity, R ≠ R²). The system measures them in the rainbow lane, not the carrier lane:
    /// RAINBOR's three waves — `path1.path2.path3`, NN · GNN · FNN, each an HTTP-0 portal acting as
    /// itself — where the anti is measured order-3 at **192 of 192**, and the free fourth zero is
    /// the fourth point at which the three waves agree. The carrier was simply the wrong instrument
    /// to ask. Operator, 2026-10-08: *"there are more electronics but wait no they are rainbons."*
    pub const fn zero_states(self) -> u8 {
        match self {
            // One reachable zero: the quiescent line, at every phase.
            Carrier::Dc => 1,
            Carrier::Ac { steps_per_cycle } => match steps_per_cycle {
                // Never cycles, so it only ever reports the dead line.
                0 => 1,
                // `half == 0`, so every phase is the rising crossing: `{Pos}` alone.
                1 => 1,
                // Rising and falling are both reachable and distinct: `{Pos, Neg}`.
                // `Nil` is NOT reachable — a crossing is never a dead line.
                _ => 2,
            },
        }
    }
}

/// 80 trits packed into one `u128`, base 3.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct TritWord(pub u128);

impl TritWord {
    pub const CAP: usize = TRITS_PER_U128;

    pub fn pack(t: &[Zero]) -> Self {
        let mut acc: u128 = 0;
        let n = if t.len() < Self::CAP {
            t.len()
        } else {
            Self::CAP
        };
        for i in 0..n {
            acc = acc * 3 + t[i].digit() as u128;
        }
        TritWord(acc)
    }

    pub fn unpack(self, out: &mut [Zero]) -> usize {
        let n = if out.len() < Self::CAP {
            out.len()
        } else {
            Self::CAP
        };
        let mut acc = self.0;
        for i in (0..n).rev() {
            out[i] = Zero::from_digit((acc % 3) as u8);
            acc /= 3;
        }
        n
    }
}

// ---------------------------------------------------------------- modular arithmetic

#[inline]
const fn mul(a: u64, b: u64) -> u64 {
    ((a as u128 * b as u128) % P as u128) as u64
}

#[inline]
const fn add(a: u64, b: u64) -> u64 {
    let s = a + b;
    if s >= P {
        s - P
    } else {
        s
    }
}

/// `base^exp mod P`, square-and-multiply. `const` so roots can be checked at compile time.
pub const fn pow(mut base: u64, mut exp: u64) -> u64 {
    let mut acc: u64 = 1;
    base %= P;
    while exp > 0 {
        if exp & 1 == 1 {
            acc = mul(acc, base);
        }
        base = mul(base, base);
        exp >>= 1;
    }
    acc
}

/// Multiplicative inverse by Fermat: `a^(P-2) mod P`. P is prime.
#[inline]
pub const fn inv(a: u64) -> u64 {
    pow(a, P - 2)
}

// ------------------------------------------------- the spherical pump wave transform

/// Forward transform: 27 samples → 27 spectral coordinates.
///
/// This is Law 20's rime prism. Newton used two prisms and showed light recombines to
/// white, which proved it composite; this is the same operation generalised to 27
/// coordinates on the sphere, over the integers so nothing is lost in the split.
///
/// `X[0]` is the DC glyph — the sum, which is the free centre of Law 0.
pub fn prism(x: &[u64; K]) -> [u64; K] {
    let mut out = [0u64; K];
    for j in 0..K {
        let wj = pow(W, j as u64);
        let mut acc = 0u64;
        let mut t = 1u64;
        for i in 0..K {
            acc = add(acc, mul(x[i] % P, t));
            t = mul(t, wj);
        }
        out[j] = acc;
    }
    out
}

/// Inverse transform: 27 spectral coordinates → the original 27 samples.
///
/// "Transfers back and forth" — the round trip is exact, not approximate, because the
/// arithmetic is integer and `27` is invertible mod `P`.
pub fn unprism(x: &[u64; K]) -> [u64; K] {
    let winv = inv(W);
    let kinv = inv(K as u64);
    let mut out = [0u64; K];
    for j in 0..K {
        let wj = pow(winv, j as u64);
        let mut acc = 0u64;
        let mut t = 1u64;
        for i in 0..K {
            acc = add(acc, mul(x[i] % P, t));
            t = mul(t, wj);
        }
        out[j] = mul(acc, kinv);
    }
    out
}

/// The pump. VIII.A.7, the Photon Law of the Rime Series: *"the more energy you push into
/// it, the further away the shell gets."*
///
/// Shell radius is the number of whole rungs of the 3-ladder the energy reaches, so it
/// grows as log₃ and never linearly. Integer only: no logarithm is taken, the rungs are
/// counted.
pub const fn pump_shell(energy: u64) -> u32 {
    let mut shell = 0u32;
    let mut rung = 1u64;
    while rung <= energy && shell < 40 {
        rung *= 3;
        shell += 1;
    }
    shell
}

/// Drive one pump step and report which zero the carrier presents at that phase.
pub const fn pump_step(c: Carrier, t: u32) -> (u32, Option<Zero>) {
    (t, c.zero_at(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_three_zeros_have_magnitude_zero() {
        assert_eq!(Zero::Neg.magnitude(), 0);
        assert_eq!(Zero::Nil.magnitude(), 0);
        assert_eq!(Zero::Pos.magnitude(), 0);
        // and they are still distinguishable
        assert_ne!(Zero::Neg.direction(), Zero::Pos.direction());
        assert_ne!(Zero::Neg, Zero::Pos);
    }

    #[test]
    fn dc_cannot_reach_the_crossings() {
        let dc = Carrier::Dc;
        assert_eq!(dc.zero_states(), 1);
        for t in 0..64 {
            assert_eq!(dc.zero_at(t), Some(Zero::Nil));
        }
        let ac = Carrier::Ac { steps_per_cycle: 4 };
        assert_eq!(ac.zero_states(), 2, "AC reaches Pos and Neg, never Nil");
        assert_eq!(ac.zero_at(0), Some(Zero::Pos));
        assert_eq!(ac.zero_at(2), Some(Zero::Neg));
        assert_eq!(ac.zero_at(1), None);
    }

    /// The old test compared `zero_states()` against a hardcoded 3 and never asked what `zero_at`
    /// could actually return. This asks, exhaustively, so the accessor cannot drift from the
    /// behaviour again.
    #[test]
    fn zero_states_equals_what_zero_at_can_reach() {
        for spc in 0..=64u32 {
            let c = Carrier::Ac {
                steps_per_cycle: spc,
            };
            let mut pos = false;
            let mut neg = false;
            let mut nil = false;
            // one full cycle covers every phase; spc == 0 has a single degenerate phase
            for t in 0..spc.max(1) {
                match c.zero_at(t) {
                    Some(Zero::Pos) => pos = true,
                    Some(Zero::Neg) => neg = true,
                    Some(Zero::Nil) => nil = true,
                    None => {}
                }
            }
            let reachable = u8::from(pos) + u8::from(neg) + u8::from(nil);
            assert_eq!(
                c.zero_states(),
                reachable,
                "zero_states disagreed with zero_at at steps_per_cycle={spc}"
            );
            if spc >= 1 {
                assert!(!nil, "AC reached Some(Nil) at steps_per_cycle={spc}");
            }
        }
        let dc = Carrier::Dc;
        assert_eq!(dc.zero_states(), 1);
        assert_eq!(dc.zero_at(7), Some(Zero::Nil));
    }

    #[test]
    fn trianti_is_order_three_and_distinct_from_its_inverse() {
        for z in [Zero::Neg, Zero::Nil, Zero::Pos] {
            assert_eq!(z.rotate().rotate().rotate(), z); // R³ = identity
            assert_ne!(z.rotate(), z.anti_rotate()); // R ≠ R²
            assert_eq!(z.rotate().anti_rotate(), z); // R·R² = identity
        }
    }

    #[test]
    fn w_really_is_a_primitive_27th_root() {
        assert_eq!(pow(G, (P - 1) / 27), W);
        assert_eq!(pow(W, 27), 1);
        for d in 1..27u64 {
            if 27 % d == 0 && d < 27 {
                assert_ne!(pow(W, d), 1, "W had order {d}, not 27");
            }
        }
    }

    #[test]
    fn the_prism_closes_on_zero() {
        // Law 20: 1 + w + ... + w²⁶ ≡ 0 (mod p)
        let mut acc = 0u64;
        for j in 0..27u64 {
            acc = add(acc, pow(W, j));
        }
        assert_eq!(acc, 0);
    }

    #[test]
    fn transfers_back_and_forth_exactly() {
        let mut x = [0u64; K];
        for i in 0..K {
            x[i] = ((i as u64 * 37 + 11) * 977) % P;
        }
        let spectrum = prism(&x);
        let back = unprism(&spectrum);
        assert_eq!(x, back, "round trip was not exact");
        // the DC glyph is the sum: the free centre
        let mut sum = 0u64;
        for i in 0..K {
            sum = add(sum, x[i]);
        }
        assert_eq!(spectrum[0], sum);
    }

    #[test]
    fn trit_word_round_trips() {
        let mut t = [Zero::Nil; TRITS_PER_U128];
        for i in 0..TRITS_PER_U128 {
            t[i] = Zero::from_digit((i % 3) as u8);
        }
        let w = TritWord::pack(&t);
        let mut back = [Zero::Nil; TRITS_PER_U128];
        assert_eq!(w.unpack(&mut back), TRITS_PER_U128);
        assert_eq!(t, back);
    }

    #[test]
    fn eighty_is_the_exact_ceiling() {
        // 3^80 fits a u128, 3^81 does not
        let mut acc: u128 = 1;
        for _ in 0..80 {
            acc = acc.checked_mul(3).expect("3^80 must fit u128");
        }
        assert!(acc.checked_mul(3).is_none(), "3^81 must overflow u128");
    }

    #[test]
    fn free_registers_are_zero_and_one() {
        assert!(Register::Zero.is_free());
        assert!(Register::Translucent.is_free());
        assert!(!Register::Red.is_free());
        assert!(!Register::Green.is_free());
        assert!(!Register::Blue.is_free());
        // the hose: translucent leads and translucent closes
        assert_eq!(Register::HOSE[0], Register::Translucent);
        assert_eq!(Register::HOSE[4], Register::Translucent);
        assert_eq!(Register::HOSE.len(), 5);
    }

    #[test]
    fn pump_shell_grows_as_log3() {
        assert_eq!(pump_shell(0), 0);
        assert_eq!(pump_shell(1), 1);
        assert_eq!(pump_shell(3), 2);
        assert_eq!(pump_shell(9), 3);
        assert_eq!(pump_shell(27), 4);
        // more energy in, further out, but never linearly
        assert!(pump_shell(1_000_000) < 20);
    }
}
