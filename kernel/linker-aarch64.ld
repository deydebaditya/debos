/* DebOS Kernel Linker Script - AArch64 */
/* For QEMU virt machine */

ENTRY(_start)

/* QEMU virt machine loads kernel at 0x40000000 */
KERNEL_START = 0x40000000;

SECTIONS {
    . = KERNEL_START;

    /* Text section - executable code */
    .text ALIGN(4K) : {
        KEEP(*(.text.boot))    /* Boot code first (contains _start) */
        *(.text .text.*)
    }

    /* Read-only data */
    .rodata ALIGN(4K) : {
        *(.rodata .rodata.*)
    }

    /* Initialized data */
    .data ALIGN(4K) : {
        *(.data .data.*)
    }

    /* Uninitialized data (BSS) */
    .bss ALIGN(4K) : {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    /* Stack section - at the end of kernel */
    .stack ALIGN(4K) (NOLOAD) : {
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
