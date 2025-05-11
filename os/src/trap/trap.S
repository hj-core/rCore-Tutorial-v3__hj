    .section .text
    .global __stvec
    .align 2
__stvec:
    call trap_handler