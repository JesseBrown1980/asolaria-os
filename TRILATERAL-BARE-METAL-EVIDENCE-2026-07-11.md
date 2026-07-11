# Trilateral evidence for Asolaria bare-metal OS — 2026-07-11

Canonical doctrine:
[`HYPER-BECHS--the-third-set/TRILATERAL-REALITY-EVIDENCE-DOCTRINE-2026-07-11.md`](https://github.com/JesseBrown1980/HYPER-BECHS--the-third-set/blob/main/TRILATERAL-REALITY-EVIDENCE-DOCTRINE-2026-07-11.md)

## Evidence ladder

Bare-metal claims require a stricter ladder than source presence alone:

```text
1. source exists and compiles for x86_64-unknown-uefi
2. EFI artifact hash is recorded
3. QEMU boots the artifact
4. independent third seat reproduces build/QEMU boot
5. specific physical machine boots from removable media
6. device-specific drivers and fabric services operate on that hardware
```

Each rung is real within its scope. A higher rung is not inferred from a lower one.

## Trilateral procedure

- Acer records source/toolchain/artifact/QEMU or hardware receipts.
- Liris or another implementation seat attacks reproducibility, boot assumptions, and safety.
- A third seat builds the immutable commit and QEMU-boots its own artifact.
- CI can compile and run automated QEMU smoke tests when available.

A third-seat QEMU success is independent evidence for the virtual boot path. It is not proof of a
successful boot on every UEFI firmware or physical device.

## Deflation errors

Reject:

- “QEMU boot is not real software”;
- “pure Rust/no_std source is only documentation”;
- “a removable-USB boot is not an operating system because Windows/Linux exists on the internal disk.”

## Inflation errors

Reject:

- compilation described as a successful boot;
- QEMU described as real-hardware validation;
- one machine's hardware boot generalized to every firmware/device;
- an armed USB described as a completed boot without a boot receipt;
- a booting kernel described as every userspace/fabric service being live;
- verification described as authorization to alter internal disks or disable security controls.

## Safety and authority

Removable-media/QEMU tests remain non-destructive by default. Physical boot, Secure-Boot changes,
driver access, and disk writes remain operator-controlled and device-specific.

## Merge rule

Merge reproducible build fixes, QEMU tests, artifact receipts, documentation, and safety hardening
when checks pass. Hold physical-device claims, destructive installers, signing-key changes, driver
privilege expansion, and cutover until their owning receipts and authority gates exist.
