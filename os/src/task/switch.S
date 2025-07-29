.altmacro

.macro save_sn n
    sd s\n, \n*8(a0)
.endm

.macro restore_sn n
    ld s\n, \n*8(a1)
.endm

    .section .text
    .global __switch
__switch:
    # __switch(curr_context: *mut TaskContext, next_context: *const TaskContext)
    beqz a0, DONE_SAVE
SAVE_CONTEXT:
    .set n, 0
    .rept 12
        save_sn %n
        .set n, n+1
    .endr
    sd ra, 12*8(a0)
    sd sp, 13*8(a0)
DONE_SAVE:

    ld x5, 15*8(a1)     # Enable the satp of next context
    csrw satp, x5
    sfence.vma

RESTORE_CONTEXT:
    .set n, 0
    .rept 12
        restore_sn %n
        .set n, n+1
    .endr
    ld ra, 12*8(a1)
    ld sp, 13*8(a1)
    ld tp, 14*8(a1)
DONE_RESTORE:

    ret