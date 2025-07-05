    .section .text.entry
    .global _start
_start:
    la sp, boot_stack_end
    call rust_main

    .section .bss.boot_stack
    .global boot_stack_start
    .global boot_stack_end
boot_stack_start:
    .space 4096 * 16
boot_stack_end: