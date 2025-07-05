OUTPUT_ARCH(riscv)
ENTRY(_start)
KERNEL_BASE = 0x80200000;

SECTIONS
{
    . = KERNEL_BASE;
    kernel_start = .;

    .text : {
        text_start = .;
        *(.text.entry)
        *(.text .text.*)
        . = ALIGN(4K);
        text_end = .;
    }

    .rodata : {
        rodata_start = .;
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
        . = ALIGN(4K);
        rodata_end = .;
    }

    .data : {
        data_start = .;
        *(.data .data.*)
        *(.sdata .sdata.*)
        . = ALIGN(4K);
        data_end = .;
    }
    
    .bss : {
        *(.bss.boot_stack)
        . = ALIGN(4K);

        user_stacks_start = .;
        *(.bss.user_stacks)
        . = ALIGN(4K);
        user_stacks_end = .;

        bss_start = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        . = ALIGN(4K);
        bss_end = .;
    }

    kernel_end = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}