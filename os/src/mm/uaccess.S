    .section .text
    .global __uaccess
    .global __uaccess_lb
    .global __uaccess_sb
__uaccess:
    # __uaccess(src: *const u8, dst: *mut u8, len: usize) -> usize
    add t5, a0, a2
Loop:
    bge a0, t5, Done
__uaccess_lb:
    lb t6, 0(a0)
__uaccess_sb:
    sb t6, 0(a1)
    addi a0, a0, 1
    addi a1, a1, 1
    j Loop
Done:
    sub a0, t5, a0
    ret

    .section .text
    .global __uaccess_fix
__uaccess_fix:
    j Done