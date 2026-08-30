# Kaname

Kaname is a minimal x86 kernel written in Rust.
Built as part of the 42 KFS project, it is a project exploring kernel development fundamentals.

The name Kaname (要) stands for "essence" / "main point" in Japanese, it is meant to be easy to read and meaningful.

---

## Features

### Implemented

- x86 bare metal target (Multiboot2 via GRUB2)
- VGA text mode (80x25)
- VGA scrollback buffer
- 8259 PIC + interrupt handling
- PS/2 keyboard driver (US QWERTY, ring buffer)
- IDT setup
- GDT setup
- Virtual terminals
- Tiny interactive shell

### Working on

- Memory paging & allocator
- Memory dumping and debug
- Kernel stack trace
- Kernel & user space
- Kernel panics

### Planned

- 8253 PIT clock
- Single-callback Kernel API
- Signal scheduler

---

## License

This project is licensed under GNU GPLv3. See [LICENSE](LICENSE) for more details.
