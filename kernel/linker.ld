/* DebOS Kernel Linker Script */
/* x86_64 kernel loaded at higher half */

ENTRY(_start)

/* Kernel is loaded at the higher half of the address space */
KERNEL_OFFSET = 0xFFFFFFFF80000000;

SECTIONS {
    /* Start at 2MB physical, mapped to higher half */
    . = KERNEL_OFFSET + 0x200000;

    /* Text section - executable code */
    .text ALIGN(4K) : AT(ADDR(.text) - KERNEL_OFFSET) {
        *(.text.boot)       /* Boot code first */
        *(.text .text.*)
    }

    /* Read-only data */
    .rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_OFFSET) {
        *(.rodata .rodata.*)
    }

    /* Initialized data */
    .data ALIGN(4K) : AT(ADDR(.data) - KERNEL_OFFSET) {
        *(.data .data.*)
    }

    /* Uninitialized data (BSS) */
    .bss ALIGN(4K) : AT(ADDR(.bss) - KERNEL_OFFSET) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    /* Stack section */
    .stack ALIGN(4K) : AT(ADDR(.stack) - KERNEL_OFFSET) {
        __stack_bottom = .;
        . += 64K;           /* 64KB kernel stack */
        __stack_top = .;
    }

    /* End of kernel */
    __kernel_end = .;

    /* Discard unnecessary sections */
    /DISCARD/ : {
        *(.comment)
        *(.eh_frame)
        *(.note.*)
    }
}
