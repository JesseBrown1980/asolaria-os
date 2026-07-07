//! Early boot diagnostics for pre-driver metal-readiness checks.
//!
//! This module is intentionally data-only: no allocation, no I/O, no pointer
//! dereference, and no unsafe. The boot loader can feed observations gathered
//! from firmware tables and receive a compact status line suitable for ConOut,
//! framebuffer text, or an unsealed HBP-style diagnostic row.

/// Secure Boot policy state visible to the loader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureBootState {
    /// The loader did not observe the Secure Boot policy variable.
    Unknown,
    /// Secure Boot policy is enabled.
    Enabled,
    /// Secure Boot policy is disabled.
    Disabled,
}

impl SecureBootState {
    /// Compact ASCII code used in diagnostic lines.
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }
}

/// Loader path class observed before handing control to the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaderPathClass {
    /// Direct firmware EFI application path.
    FirmwareEfi,
    /// Windows Boot Manager chainload path.
    WindowsBootManager,
    /// Path was unavailable or did not match a known class.
    Unknown,
}

impl LoaderPathClass {
    /// Compact ASCII code used in diagnostic lines.
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::FirmwareEfi => "firmware-efi",
            Self::WindowsBootManager => "windows-bootmgr",
            Self::Unknown => "unknown",
        }
    }
}

/// UEFI GOP pixel format reduced to early-boot-safe classifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// PixelRedGreenBlueReserved8BitPerColor.
    Rgb,
    /// PixelBlueGreenRedReserved8BitPerColor.
    Bgr,
    /// PixelBitMask.
    Bitmask,
    /// PixelBltOnly; no direct linear framebuffer is promised.
    BltOnly,
    /// Format was unavailable or not recognized.
    Unknown,
}

impl PixelFormat {
    /// Compact ASCII code used in diagnostic lines.
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Rgb => "rgb",
            Self::Bgr => "bgr",
            Self::Bitmask => "bitmask",
            Self::BltOnly => "blt-only",
            Self::Unknown => "unknown",
        }
    }

    /// Returns true when the format can back a direct framebuffer write path.
    pub const fn is_direct_framebuffer(self) -> bool {
        matches!(self, Self::Rgb | Self::Bgr | Self::Bitmask)
    }
}

/// Framebuffer facts available after GOP discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramebufferObservation {
    /// Horizontal pixels.
    pub width: u32,
    /// Vertical pixels.
    pub height: u32,
    /// GOP pixel format.
    pub pixel_format: PixelFormat,
    /// Total linear framebuffer bytes exposed to the loader.
    pub byte_len: usize,
    /// Firmware provided a non-null framebuffer base address.
    pub base_present: bool,
    /// Framebuffer stride and byte length are large enough for the reported mode.
    pub layout_verified: bool,
}

impl FramebufferObservation {
    /// Empty framebuffer observation.
    pub const ABSENT: Self = Self {
        width: 0,
        height: 0,
        pixel_format: PixelFormat::Unknown,
        byte_len: 0,
        base_present: false,
        layout_verified: false,
    };

    /// Build a framebuffer observation from firmware-provided values.
    pub const fn new(
        width: u32,
        height: u32,
        pixel_format: PixelFormat,
        byte_len: usize,
        base_present: bool,
        layout_verified: bool,
    ) -> Self {
        Self {
            width,
            height,
            pixel_format,
            byte_len,
            base_present,
            layout_verified,
        }
    }

    /// Returns true when a direct linear framebuffer is sufficiently described.
    pub const fn is_direct_complete(self) -> bool {
        self.width != 0
            && self.height != 0
            && self.byte_len != 0
            && self.base_present
            && self.layout_verified
            && self.pixel_format.is_direct_framebuffer()
    }
}

/// Raw early boot observations available before real kernel drivers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootObservations {
    /// UEFI system table was present.
    pub uefi_system_table_present: bool,
    /// UEFI ConOut handle was present.
    pub con_out_present: bool,
    /// UEFI BootServices pointer was present.
    pub boot_services_present: bool,
    /// Graphics Output Protocol was located.
    pub gop_located: bool,
    /// Framebuffer dimensions, format, and byte length.
    pub framebuffer: FramebufferObservation,
    /// Secure Boot policy observation.
    pub secure_boot: SecureBootState,
    /// Loader path classification.
    pub loader_path: LoaderPathClass,
}

impl BootObservations {
    /// Empty observation set.
    pub const EMPTY: Self = Self {
        uefi_system_table_present: false,
        con_out_present: false,
        boot_services_present: false,
        gop_located: false,
        framebuffer: FramebufferObservation::ABSENT,
        secure_boot: SecureBootState::Unknown,
        loader_path: LoaderPathClass::Unknown,
    };

    /// Returns true when GOP and a writable direct framebuffer are both observed.
    pub const fn has_direct_framebuffer(self) -> bool {
        self.gop_located && self.framebuffer.is_direct_complete()
    }
}

/// Metal-readiness stage inferred from early boot observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootStage {
    /// UEFI system table was not observed.
    NoUefi,
    /// UEFI system table was observed, but no stronger boot surface is known.
    UefiTable,
    /// UEFI text console is available.
    UefiConsole,
    /// UEFI BootServices are available for protocol discovery.
    BootServices,
    /// GOP was located, but a direct framebuffer is not complete.
    GraphicsOutput,
    /// Direct framebuffer is complete, but direct metal policy/path is unproven.
    Framebuffer,
    /// Direct framebuffer and firmware EFI path are present; Secure Boot is unknown.
    MetalCandidate,
    /// Direct framebuffer, firmware EFI path, and disabled Secure Boot are present before native drivers.
    MetalReady,
    /// Direct framebuffer exists, but policy or chainload path needs attention.
    PolicyGuarded,
}

impl BootStage {
    /// Compact ASCII code used in diagnostic lines.
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::NoUefi => "no-uefi",
            Self::UefiTable => "uefi-table",
            Self::UefiConsole => "uefi-console",
            Self::BootServices => "boot-services",
            Self::GraphicsOutput => "graphics-output",
            Self::Framebuffer => "framebuffer",
            Self::MetalCandidate => "metal-candidate",
            Self::MetalReady => "pre-driver-ready",
            Self::PolicyGuarded => "policy-guarded",
        }
    }
}

/// Classify the strongest metal-readiness stage supported by the observations.
pub const fn classify_boot_stage(observations: &BootObservations) -> BootStage {
    if !observations.uefi_system_table_present {
        return BootStage::NoUefi;
    }

    if observations.has_direct_framebuffer() {
        match (observations.loader_path, observations.secure_boot) {
            (LoaderPathClass::FirmwareEfi, SecureBootState::Disabled) => BootStage::MetalReady,
            (LoaderPathClass::FirmwareEfi, SecureBootState::Unknown) => BootStage::MetalCandidate,
            (LoaderPathClass::WindowsBootManager, _) | (_, SecureBootState::Enabled) => {
                BootStage::PolicyGuarded
            }
            (LoaderPathClass::Unknown, _) => BootStage::Framebuffer,
        }
    } else if observations.gop_located {
        BootStage::GraphicsOutput
    } else if observations.boot_services_present {
        BootStage::BootServices
    } else if observations.con_out_present {
        BootStage::UefiConsole
    } else {
        BootStage::UefiTable
    }
}

/// Status-line rendering error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderError {
    /// Caller-provided buffer was too small. `required_len` excludes any trailing NUL.
    BufferTooSmall {
        /// Number of bytes required to render the full status line.
        required_len: usize,
    },
}

/// Return the exact byte length required by [`render_status_line`].
pub fn status_line_len(observations: &BootObservations) -> usize {
    let mut writer = StatusLineWriter::count_only();
    render_status_line_inner(observations, &mut writer);
    writer.required_len()
}

/// Render a compact ASCII/HBP-ish status line into `out`.
///
/// The returned length excludes any trailing NUL; this function does not write
/// a trailing NUL. If the buffer is too small, the prefix that fits may be
/// written and the required full length is returned in [`RenderError`].
pub fn render_status_line(
    observations: &BootObservations,
    out: &mut [u8],
) -> Result<usize, RenderError> {
    let mut writer = StatusLineWriter::with_buffer(out);
    render_status_line_inner(observations, &mut writer);
    writer.finish()
}

fn render_status_line_inner(observations: &BootObservations, writer: &mut StatusLineWriter<'_>) {
    writer.push_str("ASOBTDIAG|format=hbpish|evidence=MEASURED_BOOT|source=uefi|seat=unknown|receipt=unsealed|tuple=boot:diag:early|stage=");
    writer.push_str(classify_boot_stage(observations).as_wire());
    writer.push_str("|uefi=");
    writer.push_bool(observations.uefi_system_table_present);
    writer.push_str("|conout=");
    writer.push_bool(observations.con_out_present);
    writer.push_str("|bs=");
    writer.push_bool(observations.boot_services_present);
    writer.push_str("|gop=");
    writer.push_bool(observations.gop_located);
    writer.push_str("|fb=");
    writer.push_u32(observations.framebuffer.width);
    writer.push_byte(b'x');
    writer.push_u32(observations.framebuffer.height);
    writer.push_byte(b'/');
    writer.push_str(observations.framebuffer.pixel_format.as_wire());
    writer.push_byte(b'/');
    writer.push_usize(observations.framebuffer.byte_len);
    writer.push_str("|fbbase=");
    writer.push_bool(observations.framebuffer.base_present);
    writer.push_str("|fblayout=");
    writer.push_bool(observations.framebuffer.layout_verified);
    writer.push_str("|sb=");
    writer.push_str(observations.secure_boot.as_wire());
    writer.push_str("|loader=");
    writer.push_str(observations.loader_path.as_wire());
}

struct StatusLineWriter<'a> {
    buffer: Option<&'a mut [u8]>,
    required: usize,
    written: usize,
}

impl<'a> StatusLineWriter<'a> {
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

    fn push_bool(&mut self, value: bool) {
        self.push_byte(if value { b'1' } else { b'0' });
    }

    fn push_str(&mut self, value: &str) {
        self.push_bytes(value.as_bytes());
    }

    fn push_byte(&mut self, value: u8) {
        self.push_bytes(&[value]);
    }

    fn push_u32(&mut self, value: u32) {
        self.push_decimal(value);
    }

    fn push_usize(&mut self, value: usize) {
        self.push_decimal(value);
    }

    fn push_decimal<T>(&mut self, value: T)
    where
        T: Decimal,
    {
        let mut digits = [0u8; 39];
        let mut n = value;
        let mut cursor = digits.len();

        if n.is_zero() {
            self.push_byte(b'0');
            return;
        }

        while !n.is_zero() {
            cursor -= 1;
            digits[cursor] = b'0' + n.rem_10();
            n = n.div_10();
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

trait Decimal: Copy {
    fn is_zero(self) -> bool;
    fn rem_10(self) -> u8;
    fn div_10(self) -> Self;
}

impl Decimal for u32 {
    fn is_zero(self) -> bool {
        self == 0
    }

    fn rem_10(self) -> u8 {
        (self % 10) as u8
    }

    fn div_10(self) -> Self {
        self / 10
    }
}

impl Decimal for usize {
    fn is_zero(self) -> bool {
        self == 0
    }

    fn rem_10(self) -> u8 {
        (self % 10) as u8
    }

    fn div_10(self) -> Self {
        self / 10
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

    const FULL_FB: FramebufferObservation =
        FramebufferObservation::new(1920, 1080, PixelFormat::Bgr, 8_294_400, true, true);

    fn observed() -> BootObservations {
        BootObservations {
            uefi_system_table_present: true,
            con_out_present: true,
            boot_services_present: true,
            gop_located: true,
            framebuffer: FULL_FB,
            secure_boot: SecureBootState::Disabled,
            loader_path: LoaderPathClass::FirmwareEfi,
        }
    }

    #[test]
    fn classifies_structural_boot_surfaces() {
        assert_eq!(
            classify_boot_stage(&BootObservations::EMPTY),
            BootStage::NoUefi
        );

        let mut obs = BootObservations {
            uefi_system_table_present: true,
            ..BootObservations::EMPTY
        };
        assert_eq!(classify_boot_stage(&obs), BootStage::UefiTable);

        obs.con_out_present = true;
        assert_eq!(classify_boot_stage(&obs), BootStage::UefiConsole);

        obs.boot_services_present = true;
        assert_eq!(classify_boot_stage(&obs), BootStage::BootServices);

        obs.gop_located = true;
        assert_eq!(classify_boot_stage(&obs), BootStage::GraphicsOutput);
    }

    #[test]
    fn classifies_framebuffer_and_metal_policy() {
        let mut obs = observed();
        assert_eq!(classify_boot_stage(&obs), BootStage::MetalReady);

        obs.secure_boot = SecureBootState::Unknown;
        assert_eq!(classify_boot_stage(&obs), BootStage::MetalCandidate);

        obs.secure_boot = SecureBootState::Enabled;
        assert_eq!(classify_boot_stage(&obs), BootStage::PolicyGuarded);

        obs.secure_boot = SecureBootState::Disabled;
        obs.loader_path = LoaderPathClass::WindowsBootManager;
        assert_eq!(classify_boot_stage(&obs), BootStage::PolicyGuarded);

        obs.loader_path = LoaderPathClass::Unknown;
        assert_eq!(classify_boot_stage(&obs), BootStage::Framebuffer);
    }

    #[test]
    fn blt_only_is_not_direct_framebuffer() {
        let obs = BootObservations {
            framebuffer: FramebufferObservation::new(
                1024,
                768,
                PixelFormat::BltOnly,
                3_145_728,
                true,
                true,
            ),
            ..observed()
        };

        assert_eq!(classify_boot_stage(&obs), BootStage::GraphicsOutput);
    }

    #[test]
    fn renders_exact_hbp_status_line() {
        let obs = observed();
        let mut out = [0u8; 320];
        let written = render_status_line(&obs, &mut out).expect("buffer is large enough");
        let line = core::str::from_utf8(&out[..written]).expect("line is ascii");

        assert_eq!(
            line,
            "ASOBTDIAG|format=hbpish|evidence=MEASURED_BOOT|source=uefi|seat=unknown|receipt=unsealed|tuple=boot:diag:early|stage=pre-driver-ready|uefi=1|conout=1|bs=1|gop=1|fb=1920x1080/bgr/8294400|fbbase=1|fblayout=1|sb=disabled|loader=firmware-efi"
        );
        assert_eq!(written, status_line_len(&obs));
    }

    #[test]
    fn render_reports_required_len_for_short_buffer() {
        let obs = observed();
        let required = status_line_len(&obs);
        let mut out = [0u8; 8];

        assert_eq!(
            render_status_line(&obs, &mut out),
            Err(RenderError::BufferTooSmall {
                required_len: required
            })
        );
        assert_eq!(&out, b"ASOBTDIA");
    }

    #[test]
    fn render_handles_zero_framebuffer_values() {
        let obs = BootObservations {
            uefi_system_table_present: true,
            gop_located: true,
            ..BootObservations::EMPTY
        };
        let mut out = [0u8; 320];
        let written = render_status_line(&obs, &mut out).expect("buffer is large enough");
        let line = core::str::from_utf8(&out[..written]).expect("line is ascii");

        assert_eq!(
            line,
            "ASOBTDIAG|format=hbpish|evidence=MEASURED_BOOT|source=uefi|seat=unknown|receipt=unsealed|tuple=boot:diag:early|stage=graphics-output|uefi=1|conout=0|bs=0|gop=1|fb=0x0/unknown/0|fbbase=0|fblayout=0|sb=unknown|loader=unknown"
        );
    }
}
