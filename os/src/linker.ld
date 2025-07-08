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
        . = ALIGN(4K);

        text_trap_start = .;
        *(.text.trap)
        . = ALIGN(4K);
        text_trap_end = .;

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

        task_kernel_stacks_start = .;
        *(.bss.task_kernel_stacks)
        . = ALIGN(4K);
        task_kernel_stacks_end = .;

        task_user_stacks_start = .;
        *(.bss.task_user_stacks)
        . = ALIGN(4K);
        task_user_stacks_end = .;

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