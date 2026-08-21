# Kaname

Kaname is a minimal x86 kernel written in Rust.
Built as part of the 42 KFS project, it is a project exploring kernel development fundamentals.

The name Kaname (要) stands for "essence" / "main point" in Japanese, it is meant to be easy to read and meaningful.

---

## Features

### Implemented

- VGA text mode (80x25)

### Working on

- x86 bare metal target (Multiboot2 via GRUB2)
- IDT setup
- 8259 PIC + interrupt handling
- PS/2 keyboard driver (US QWERTY, ring buffer)
- Virtual terminals
- VGA scrollback buffer

### Planned

- GDT setup
- 8253 PIT clock
- Kernel stack trace
- Tiny interactive shell
- Memory paging & allocator
- Kernel & user space
- Kernel panics
- Memory dumping and debug
- Single-callback Kernel API
- Signal scheduler

---

## License

This project is licensed under GNU GPLv3. See [LICENSE](LICENSE) for more details.
