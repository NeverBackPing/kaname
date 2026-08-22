# Constant

.set ALIGN, 1 << 0              # Align loaded modules on page boundaries
.set MEMINFO, 1 << 1            # Request memory information from GRUB
.set FLAGS, ALIGN | MEMINFO     # Combine Multiboot flags
.set MAGIC, 0x1BADB002          # Multiboot addr
.set CHECKSUM, -(MAGIC + FLAGS) # Ensure the header checksum equals 0

.section .multiboot, "a"
.align 4
.long MAGIC
.long FLAGS
.long CHECKSUM


.section .text

.section .rodata

.section .bss
