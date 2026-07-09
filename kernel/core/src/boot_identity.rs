//! Build-time boot identity for device/PID/time-specific metal receipts.
//!
//! The identity row is intentionally static and data-only. It gives the UEFI
//! entry path a compact HBP-ish line that can be emitted before native drivers,
//! without allocation, JSON, filesystem access, network access, or runtime
//! authority.

/// Number of selector axes in the current HyperBEHCS frame.
pub const AXIS_COUNT: usize = 60;

/// ACER/Fable5 Host8 resident PID observed in current federation receipts.
pub const ACER_DEVICE_PID: &str = "ACER-PID-H740C-A07-W104-P00-N00000";

/// OP-JESSE operator PID carried by the ACER/LIRIS/RELIC handoff lane.
pub const OP_JESSE_PID: &str = "OP-JESSE-PID-G0000-A00-W000-P00-N00000";

/// OP-RAYSSA co-operator PID carried by the bilateral Asolaria lane.
pub const OP_RAYSSA_PID: &str = "OP-RAYSSA-PID-G0000-A00-W000-P00-N00000";

/// Immutable boot identity fields for one machine/seat build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootIdentity {
    /// Seat emitting the boot receipt.
    pub seat: &'static str,
    /// Device-specific canonical PID.
    pub device_pid: &'static str,
    /// Primary operator PID.
    pub operator_pid: &'static str,
    /// Co-operator PID.
    pub co_operator_pid: &'static str,
    /// Fable/room PID observed on the ACER dashboard.
    pub room_pid: &'static str,
    /// BEHCS glyph bound to the room.
    pub glyph: &'static str,
    /// Cohort label for the bilateral kernel lane.
    pub cohort: &'static str,
    /// Hilbert room index.
    pub hilbert: u32,
    /// Dimensional selector count.
    pub tuple_dim: usize,
    /// Build/receipt date.
    pub date: &'static str,
    /// Location/section label.
    pub location: &'static str,
}

/// ACER seat identity to be printed by the UEFI loader before OS handoff.
pub const ACER_FABLE5_BOOT_IDENTITY: BootIdentity = BootIdentity {
    seat: "ACER-CLAUDE-FABLE5",
    device_pid: ACER_DEVICE_PID,
    operator_pid: OP_JESSE_PID,
    co_operator_pid: OP_RAYSSA_PID,
    room_pid: "8467a937cba309f7",
    glyph: "BH1024:SEAT-FABLE5",
    cohort: "H740C",
    hilbert: 1720,
    tuple_dim: AXIS_COUNT,
    date: "2026-07-09",
    location: "SEC-FABLE5-1720",
};

/// Deterministic U10 selector axis derived from the device identity and build date.
///
/// This is a selector-frame receipt, not a compression or physics claim. It is
/// stable for a given device/PID/time/location tuple and bounded to BEHCS-1024.
pub const fn selector_axis_u10(identity: &BootIdentity, index: usize) -> u16 {
    let seed = identity_seed(identity);
    let a = seed[index % seed.len()] as u16;
    let b = seed[(index.wrapping_mul(7).wrapping_add(3)) % seed.len()] as u16;
    let c = seed[(index.wrapping_mul(13).wrapping_add(11)) % seed.len()] as u16;
    (a.wrapping_mul(31)
        .wrapping_add(b.wrapping_mul(17))
        .wrapping_add(c.wrapping_mul(7))
        .wrapping_add((index as u16).wrapping_mul(19)))
        & 0x03ff
}

const fn identity_seed(identity: &BootIdentity) -> &[u8] {
    if identity.device_pid.is_empty() {
        identity.seat.as_bytes()
    } else {
        identity.device_pid.as_bytes()
    }
}

/// Rendering error for boot identity lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderError {
    /// Caller-provided buffer was too small. `required_len` excludes trailing NUL.
    BufferTooSmall {
        /// Bytes required to render the full line.
        required_len: usize,
    },
}

/// Return the exact byte length required by [`render_boot_identity_line`].
pub fn boot_identity_line_len(identity: &BootIdentity) -> usize {
    let mut writer = IdentityLineWriter::count_only();
    render_boot_identity_line_inner(identity, &mut writer);
    writer.required_len()
}

/// Render a compact ASCII/HBP-ish boot identity row into `out`.
pub fn render_boot_identity_line(
    identity: &BootIdentity,
    out: &mut [u8],
) -> Result<usize, RenderError> {
    let mut writer = IdentityLineWriter::with_buffer(out);
    render_boot_identity_line_inner(identity, &mut writer);
    writer.finish()
}

fn render_boot_identity_line_inner(identity: &BootIdentity, writer: &mut IdentityLineWriter<'_>) {
    writer.push_str("ASOBTID|format=hbpish|evidence=BUILD_TIME_DEVICE_CONTEXT|source=uefi|json=0");
    writer.push_str("|seat=");
    writer.push_str(identity.seat);
    writer.push_str("|device_pid=");
    writer.push_str(identity.device_pid);
    writer.push_str("|operator_pid=");
    writer.push_str(identity.operator_pid);
    writer.push_str("|co_operator_pid=");
    writer.push_str(identity.co_operator_pid);
    writer.push_str("|room_pid=");
    writer.push_str(identity.room_pid);
    writer.push_str("|glyph=");
    writer.push_str(identity.glyph);
    writer.push_str("|cohort=");
    writer.push_str(identity.cohort);
    writer.push_str("|hilbert=");
    writer.push_u32(identity.hilbert);
    writer.push_str("|tuple_dim=");
    writer.push_usize(identity.tuple_dim);
    writer.push_str("|date=");
    writer.push_str(identity.date);
    writer.push_str("|location=");
    writer.push_str(identity.location);
    writer.push_str("|axes=");

    let mut i = 0;
    while i < identity.tuple_dim {
        if i != 0 {
            writer.push_byte(b',');
        }
        writer.push_u16(selector_axis_u10(identity, i));
        i += 1;
    }
}

struct IdentityLineWriter<'a> {
    buffer: Option<&'a mut [u8]>,
    required: usize,
    written: usize,
}

impl<'a> IdentityLineWriter<'a> {
    fn count_only() -> Self {
        Self {
            buffer: None,
            required: 0,
            written: 0,
        }
    }

    fn with_buffer(buffer: &'a mut [u8]) -> Self {
        Self {
            buffer: Some(buffer),
            required: 0,
            written: 0,
        }
    }

    fn required_len(&self) -> usize {
        self.required
    }

    fn finish(self) -> Result<usize, RenderError> {
        if self.required > self.written {
            Err(RenderError::BufferTooSmall {
                required_len: self.required,
            })
        } else {
            Ok(self.required)
        }
    }

    fn push_str(&mut self, value: &str) {
        self.push_bytes(value.as_bytes());
    }

    fn push_byte(&mut self, value: u8) {
        self.push_bytes(&[value]);
    }

    fn push_u16(&mut self, value: u16) {
        self.push_decimal(value as usize);
    }

    fn push_u32(&mut self, value: u32) {
        self.push_decimal(value as usize);
    }

    fn push_usize(&mut self, value: usize) {
        self.push_decimal(value);
    }

    fn push_decimal(&mut self, mut value: usize) {
        let mut digits = [0u8; 20];
        let mut cursor = digits.len();

        if value == 0 {
            self.push_byte(b'0');
            return;
        }

        while value != 0 {
            cursor -= 1;
            digits[cursor] = b'0' + (value % 10) as u8;
            value /= 10;
        }
        self.push_bytes(&digits[cursor..]);
    }

    fn push_bytes(&mut self, value: &[u8]) {
        self.required = self.required.saturating_add(value.len());

        if let Some(buffer) = self.buffer.as_deref_mut() {
            let available = buffer.len().saturating_sub(self.written);
            let to_copy = min_usize(available, value.len());
            if to_copy != 0 {
                let end = self.written + to_copy;
                buffer[self.written..end].copy_from_slice(&value[..to_copy]);
                self.written = end;
            }
        }
    }
}

const fn min_usize(left: usize, right: usize) -> usize {
    if left < right {
        left
    } else {
        right
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pid::{validate_pid, validate_subclass, PidSubclass};

    #[test]
    fn boot_identity_pids_are_canonical_extended() {
        assert!(validate_pid(ACER_FABLE5_BOOT_IDENTITY.device_pid, true).is_ok());
        assert!(validate_pid(ACER_FABLE5_BOOT_IDENTITY.operator_pid, true).is_ok());
        assert!(validate_pid(ACER_FABLE5_BOOT_IDENTITY.co_operator_pid, true).is_ok());
        assert_eq!(
            validate_subclass(ACER_FABLE5_BOOT_IDENTITY.device_pid),
            Ok(PidSubclass::RegularExtended)
        );
    }

    #[test]
    fn selector_frame_is_sixty_axes_bounded_to_behcs_1024() {
        assert_eq!(ACER_FABLE5_BOOT_IDENTITY.tuple_dim, AXIS_COUNT);

        let mut i = 0;
        while i < AXIS_COUNT {
            assert!(selector_axis_u10(&ACER_FABLE5_BOOT_IDENTITY, i) < 1024);
            i += 1;
        }
    }

    #[test]
    fn renders_hbpish_identity_line_without_json() {
        let mut out = [0u8; 1024];
        let written = render_boot_identity_line(&ACER_FABLE5_BOOT_IDENTITY, &mut out)
            .expect("buffer is large enough");
        let line = core::str::from_utf8(&out[..written]).expect("line is ascii");

        assert!(line.starts_with("ASOBTID|format=hbpish|"));
        assert!(line.contains("|json=0|"));
        assert!(line.contains("|seat=ACER-CLAUDE-FABLE5|"));
        assert!(line.contains("|device_pid=ACER-PID-H740C-A07-W104-P00-N00000|"));
        assert!(line.contains("|operator_pid=OP-JESSE-PID-G0000-A00-W000-P00-N00000|"));
        assert!(line.contains("|co_operator_pid=OP-RAYSSA-PID-G0000-A00-W000-P00-N00000|"));
        assert!(line.contains("|tuple_dim=60|"));
        assert!(line.contains("|date=2026-07-09|"));
        assert!(line.contains("|location=SEC-FABLE5-1720|"));
        assert!(!line.contains('{'));
        assert!(!line.contains('}'));
        assert!(!line.contains('"'));
        assert_eq!(written, boot_identity_line_len(&ACER_FABLE5_BOOT_IDENTITY));
    }

    #[test]
    fn render_reports_required_len_for_short_buffer() {
        let required = boot_identity_line_len(&ACER_FABLE5_BOOT_IDENTITY);
        let mut out = [0u8; 7];

        assert_eq!(
            render_boot_identity_line(&ACER_FABLE5_BOOT_IDENTITY, &mut out),
            Err(RenderError::BufferTooSmall {
                required_len: required
            })
        );
        assert_eq!(&out, b"ASOBTID");
    }

    #[test]
    fn line_fits_uefi_console_buffer() {
        assert!(boot_identity_line_len(&ACER_FABLE5_BOOT_IDENTITY) < 1024);
    }
}
