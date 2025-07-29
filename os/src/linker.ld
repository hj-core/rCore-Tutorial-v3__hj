OUTPUT_ARCH(riscv)
ENTRY(_start)

KERNEL_BASE_PA = 0x80200000;
KERNEL_VA_OFFSET = 0xffffffc000000000;
KERNEL_BASE_VA = KERNEL_BASE_PA + KERNEL_VA_OFFSET;

SECTIONS
{
    . = KERNEL_BASE_VA;
    kernel_start = .;

    text_start = .;
    .text : AT(. - KERNEL_VA_OFFSET) {
        *(.text.entry)
        *(.text .text.*)
    }
    . = ALIGN(4K);
    text_end = .;

    boot_pgt = .;
    . += 4K;

    kernel_stack_start = .;
    . += 16K;
    kernel_stack_end = .;

    rodata_start = .;
    .rodata : AT(. - KERNEL_VA_OFFSET) {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(4K);
    rodata_end = .;

    data_start = .;
    .data : AT(. - KERNEL_VA_OFFSET) {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    . = ALIGN(4K);
    data_end = .;
    
    bss_start = .;
    .bss : AT(. - KERNEL_VA_OFFSET) {
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