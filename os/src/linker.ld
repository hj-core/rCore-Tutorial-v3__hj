OUTPUT_ARCH(riscv)
ENTRY(_start)
KERNEL_BASE = 0x80200000;

SECTIONS
{
    . = KERNEL_BASE;
    kernel_start = .;
    text_start = .;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    . = ALIGN(4K);
    text_end = .;
    rodata_start = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(4K);
    rodata_end = .;
    data_start = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(4K);
    data_end = .;
    bss_start = .;
    .bss : {
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    . = ALIGN(4K);
    bss_end = .;
    kernel_end = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}