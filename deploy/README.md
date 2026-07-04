# Make a bootable Asolaria USB

You have `asolaria-os.efi` (from `kernel/scripts/build-img.sh x86_64`, or the cargo build → `kernel/target/x86_64-unknown-uefi/release/asolaria-os.efi`). Put it on a USB as an **EFI System Partition** and your firmware can boot it.

## The simple, safe way (any OS)

An EFI System Partition is just a **FAT32** partition with the loader at `EFI/BOOT/BOOTX64.EFI`.

### Linux
```sh
# WARNING: replace /dev/sdX1 with YOUR usb partition (check `lsblk`) — this erases that partition
sudo mkfs.vfat -F32 /dev/sdX1
sudo mount /dev/sdX1 /mnt
sudo mkdir -p /mnt/EFI/BOOT
sudo cp asolaria-os.efi /mnt/EFI/BOOT/BOOTX64.EFI
sudo umount /mnt
```

### Windows
- Format the USB (or a partition) as **FAT32**.
- Create the folder `EFI\BOOT\` on it.
- Copy `asolaria-os.efi` there, renamed to `BOOTX64.EFI`.

## Boot it
1. Reboot; open the firmware boot menu (commonly **F12**; some machines **F10 / Esc / F2**).
2. Pick the **UEFI: &lt;your USB&gt;** entry.
3. If it doesn't load, enter BIOS setup and set **Secure Boot → Disabled** (an unsigned `.efi` won't load otherwise), save, retry.
4. Asolaria boots.

**Reversible + non-destructive:** this only writes to the USB. Your internal disk and its OS are untouched — unplug the USB and reboot to return to normal.

> Advanced: some setups use a *vault-safe raw-write* deploy (append the ESP image to a disk's free tail and splice a single MBR partition entry, leaving existing partitions byte-identical). That path is machine-specific and risky; for almost everyone the FAT32 ESP above is the right, safe way.
