ENTRY(_start)

PHDRS
{
  gdt    PT_LOAD FLAGS(4); /* R */
  boot   PT_LOAD FLAGS(5); /* R+X */
  rodata PT_LOAD FLAGS(4); /* R */
  data   PT_LOAD FLAGS(6); /* R+W */
}

SECTIONS {
  . = 0x800;
  .gdt : {
    KEEP(*(.gdt))
  } :gdt

  /* Kernel load address */
  . = 1M;

  __kernel_start = .;

  /*
  First put the multiboot header
  */
  .boot ALIGN(4) : {
    KEEP(*(.multiboot))
  } :boot

  .text : ALIGN(4K) {
    *(.text)
    *(.text.*)
  } :boot

  __data_start = .;
  /* Read-only data. */
  .rodata : ALIGN(4K) {
    *(.rodata)
    *(.rodata.*)
  } :rodata

  /* Read-write data (initialized) */
  .data : ALIGN(4K) {
    *(.data)
    *(.data.*)
  } :data

  /* Read-write data (uninitialized) and stack */
  .bss : ALIGN(4K) {
    _bss_start = .;
    *(COMMON)
    *(.bss)
    *(.bss.*)
    _bss_end = .;
  } :data
  __data_end = .;

  __kernel_end = .;

  /DISCARD/ : {
    *(.eh_frame*)
  }
}
