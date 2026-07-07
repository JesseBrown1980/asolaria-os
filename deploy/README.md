# Make a bootable Asolaria USB

You have `asolaria-os.efi` from `kernel/scripts/build-img.sh x86_64`, or from the Cargo build at `kernel/target/x86_64-unknown-uefi/release/asolaria-os.efi`. Put it on a verified EFI System Partition and firmware can boot it.

## Hard safety gate

Do not run these commands against SOVLINUX-2TB or any unknown USB/disk. In the Asolaria federation, USB writes and formatting require operator-witness plus the current tool-advisor/seat check. Windows partition labels are not authoritative.

The examples below are only for a verified sacrificial USB partition that the operator explicitly permits erasing.

## ESP layout

An EFI System Partition is a FAT32 partition with the loader at `EFI/BOOT/BOOTX64.EFI`.

### Linux

```sh
# Replace /dev/sdX1 only after checking lsblk and confirming this is NOT SOVLINUX.
# This erases that partition.
sudo mkfs.vfat -F32 /dev/sdX1
sudo mount /dev/sdX1 /mnt
sudo mkdir -p /mnt/EFI/BOOT
sudo cp asolaria-os.efi /mnt/EFI/BOOT/BOOTX64.EFI
sudo umount /mnt
```

### Windows

- Confirm the target is a sacrificial USB/partition and not SOVLINUX.
- Format that partition as FAT32 only after operator witness.
- Create `EFI\BOOT\` on it.
- Copy `asolaria-os.efi` there, renamed to `BOOTX64.EFI`.

## Boot it

1. Reboot; open the firmware boot menu, commonly F12, F10, Esc, or F2.
2. Pick the `UEFI: <your USB>` entry.
3. If it does not load, enter BIOS setup and disable Secure Boot. An unsigned `.efi` will not load under Secure Boot.
4. Asolaria boots to the current early kernel surface.

For a verified disposable USB only, this is reversible: unplug the USB and reboot to return to the normal internal disk path. It is not a general statement that USB writes are safe.

Advanced paths such as raw tail-write, MBR splice, internal ESP edits, BCD, NVRAM boot entries, or SOVLINUX substrate writes are machine-specific hard-gated operations. They are not covered by this README.