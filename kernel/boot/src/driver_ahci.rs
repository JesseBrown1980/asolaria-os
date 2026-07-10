//! SATA AHCI storage driver — device-gated + time-stamped SCAFFOLD.
//!
//! DEVICE-specific: engages only when the device's tuple shows a SATA AHCI controller
//! (PCI mass-storage class `0x01`, subclass `0x06`; `hwinv` `ahci=1`) — e.g. relic's Intel
//! `8086:1E03`. On a machine whose disk is behind Intel RST/VMD instead (acer/liris, `8086:282A`)
//! this reports `NOT_APPLICABLE`, and `driver_rst_vmd` takes that machine's storage path.
//! TIME-specific: every driver event is stamped with this boot's `boot_pid` + UTC.
//!
//! Both storage drivers ship in the ONE shared kernel (trilateral build); the boot selects the
//! applicable path from live PCI presence. SCAFFOLD ONLY (E=0, `writes=0`): emits the gate + intent.
//! The real work — AHCI HBA reset, port enumeration, and command-list/FIS bring-up to reach the
//! SATA disk — is the tracked next step and requires operator-witnessed matched metal to verify.

use crate::hwinv::{put_bytes, put_hex8, HwSummary};
use crate::rtc::BootTime;
use crate::serial_print;

/// Emit the device-gated + time-stamped BOOTDRIVER scaffold row for the AHCI path.
pub(crate) fn probe(hw: &HwSummary, boot_pid: &[u8; 8], bt: &BootTime, seat: &[u8]) {
    let mut row = [0u8; 256];
    let mut i = put_bytes(
        &mut row,
        0,
        b"  BOOTDRIVER|driver=sata-ahci|match=class01:06|device_pid=",
    );
    for &b in &hw.device_digest[..8] {
        i = put_hex8(&mut row, i, b);
    }
    i = put_bytes(&mut row, i, b"|boot_pid=");
    for &b in boot_pid {
        i = put_hex8(&mut row, i, b);
    }
    i = put_bytes(&mut row, i, b"|utc=");
    i += bt.write_utc(&mut row[i..]);
    i = put_bytes(&mut row, i, b"|seat=");
    i = put_bytes(&mut row, i, seat);
    i = put_bytes(&mut row, i, b"|status=");
    if hw.ahci {
        i = put_bytes(&mut row, i, b"GATED_MATCH|ahci_bringup=TODO_next_step");
    } else {
        i = put_bytes(&mut row, i, b"NOT_APPLICABLE|no_sata_ahci_on_this_device");
    }
    i = put_bytes(&mut row, i, b"|writes=0|e=0|fire=0|json=0\r\n");
    // SAFETY: serial_print writes COM1 via port I/O; no memory or device-state hazard.
    unsafe {
        serial_print(&row[..i]);
    }
}
