# Asolaria OS

A real operating system on bare metal, in pure Rust. Not a hosted app — a `#![no_std] #![no_main]` **UEFI microkernel** that boots the machine directly, with no Windows or Linux underneath.

**The whole system, one repo.** Clone it, build the bootable `.efi`, run it in QEMU, boot it on real hardware.

## What's in here

- **`kernel/`** — the OS itself.
  - `kernel/boot` — the `#![no_std] #![no_main]` UEFI application (`x86_64-unknown-uefi`) → produces **`asolaria-os.efi`**, a bootable EFI binary.
  - `kernel/core` — the microkernel: `pid`, `syscall`, `vfs`, `crypto` (ed25519), `frame_alloc`, `spawn_gate`, `sign_gate`, `agent_runtime`, `bus_fabric`, `glyph_genesis`, `tier`, `link_auth`, …
  - `kernel/scripts` — `build-img.sh` (build the `.efi`), `qemu-test.sh` (boot it in QEMU), `sbom.sh`.
- **`surface/`** — the ASI OS human front-end (also pure Rust): terminal shells, a live fabric status strip, your own local key. The userland you see.
- **`kernel/rust-toolchain.toml`** — pins Rust **1.81** so it builds reproducibly (the crypto deps don't compile on the newest rustc — the one gotcha, handled for you).

## Build + run in QEMU (no hardware needed)

Requires `rustup` (the pin fetches 1.81 automatically) and `qemu-system-x86_64`.

```sh
git clone https://github.com/JesseBrown1980/asolaria-os
cd asolaria-os/kernel
cargo build --release --target x86_64-unknown-uefi --bin asolaria-os
#   -> target/x86_64-unknown-uefi/release/asolaria-os.efi   (a real bootable OS)
scripts/qemu-test.sh          # boots the .efi in QEMU
```

(`scripts/build-img.sh x86_64` does the build and drops a sha-stamped copy in `kernel/dist/`.)

## Boot it on your own metal

1. Build `asolaria-os.efi` (above).
2. Make a bootable USB — see **[`deploy/`](deploy/)**. The simple, safe way: a small FAT32 partition marked as an EFI System Partition, with the `.efi` copied to `EFI/BOOT/BOOTX64.EFI`.
3. Reboot → boot menu (often **F12**) → pick the **UEFI USB**. Disable **Secure Boot** if the `.efi` is unsigned.
4. Asolaria boots on metal, before anything else. **Unplug the USB to boot your normal OS again — nothing on your internal disk is touched.**

## The surface

`surface/` runs the ASI OS front-end. On a hosted OS: `sh surface/scripts/install.sh` → `http://127.0.0.1:4600`. You mint your own local key; nothing leaves your machine. (See `surface/README.md` and `surface/FABRIC-NODE.md` for a full fabric node — recall + the 8-byte host.)

## Honest boundary

- **Builds + boots in QEMU** — proven: `asolaria-os.efi` compiles with 1.81 and QEMU-boots.
- **Boots on real hardware** — the same `.efi` is armed on the author's USB; the first bare-metal boot on any given machine is the real test (UEFI + Secure-Boot behavior varies by firmware). It is **non-destructive** — a removable USB; unplug it to go back.
- The deeper architecture docs (driver model, userspace ABI, phase history) live in [asolaria-federation-1024](https://github.com/JesseBrown1980/asolaria-federation-1024), the full workspace this kernel is part of.

## License

MIT OR Apache-2.0. Build your own.
