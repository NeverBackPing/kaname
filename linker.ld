ENTRY(_start)

SECTIONS {
  /* Kernel load address */
  . = 1M;

  __kernel_start = .;

  /*
  First put the multiboot header
  */
  .boot ALIGN(4) : {
    KEEP(*(.multiboot))
  }

  .text : ALIGN(4K) {
    *(.text)
    *(.text.*)
  }

  __data_start = .;
  /* Read-only data. */
  .rodata : ALIGN(4K) {
    *(.rodata)
    *(.rodata.*)
  }

  /* Read-write data (initialized) */
  .data : ALIGN(4K) {
    *(.data)
    *(.data.*)
  }

  /* Read-write data (uninitialized) and stack */
  .bss : ALIGN(4K) {
    _bss_start = .;
    *(COMMON)
    *(.bss)
    *(.bss.*)
    _bss_end = .;
  }
  __data_end = .;

  __kernel_end = .;
}
