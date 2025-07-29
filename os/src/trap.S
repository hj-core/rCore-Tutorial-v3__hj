.altmacro

.macro save_gp n
    sd x\n, \n*8(tp)
.endm

.macro restore_gp n
    ld x\n, \n*8(tp)
.endm

    .global __stvec
    .global __restore
    .align 2                   # The lowest two bits are always zero and are overridden by mode
__stvec:
    csrrw tp, sscratch, tp

    /* If tp == 0, we are in kernel space and do not need to
    * store the context. Otherwise, tp holds the address of
    * the trap context, where we save our context, and it is also
    * the current kernel sp.
    */
    beqz tp, DONE_SAVE

SAVE_CONTEXT:
    .set n, 0
    .rept 32
        save_gp %n
        .set n, n+1
    .endr
    csrrw x5, sscratch, x0
    sd x5, 4*8(tp)
    csrr x5, sstatus
    sd x5, 32*8(tp)
    csrr x5, sepc
    sd x5, 33*8(tp)
    mv sp, tp
DONE_SAVE:

    mv a0, tp                  # Set the arugment for trap_handler, i.e., a pointer to the saved context
    call trap_handler

__restore:
    bnez tp, RESTORE_CONTEXT   # Same logic as the save_context
    csrrw tp, sscratch, tp
    sret

RESTORE_CONTEXT:
    csrw sscratch, tp
    ld x5, 32*8(tp)
    csrw sstatus, x5
    ld x5, 33*8(tp)
    csrw sepc, x5
    .set n, 5
    .rept 27
        restore_gp %n
        .set n, n+1
    .endr
    restore_gp 1
    restore_gp 2
    restore_gp 3
    restore_gp 4
DONE_RESTORE:

    sret
