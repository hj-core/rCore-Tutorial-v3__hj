OUTPUT_ARCH(riscv)
ENTRY(_start)
APP_BASE = 0x80400000;

SECTIONS
{
    . = APP_BASE;
    
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }
    . = ALIGN(4K);

    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(4K);

    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    . = ALIGN(4K);

    bss_start = .;
    .bss : {
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
    . = ALIGN(4K);
    bss_end = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}