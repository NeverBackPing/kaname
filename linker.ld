ENTRY(_start)

SECTIONS
{
    /* Adresse de chargement du kernel*/
	. = 2M;

    __kernel_start = .;

	/* 
        First put the multiboot header

    */
    .boot BLOCK(4K) : ALIGN(4K)
    {
		*(.multiboot)
    }

	.text BLOCK(4K) : ALIGN(4K)
	{
		*(.text)
	}

    __data_start = .;
	/* Read-only data. */
	.rodata BLOCK(4K) : ALIGN(4K)
	{
		*(.rodata)
	}

	/* Read-write data (initialized) */
	.data BLOCK(4K) : ALIGN(4K)
	{
		*(.data)
	}

	/* Read-write data (uninitialized) and stack */
	.bss BLOCK(4K) : ALIGN(4K)
	{
        _bss_start = .;
		*(COMMON)
		*(.bss)
        _bss_end = .;
	}
    __data_end = .;

    __kernel_end = .;
}