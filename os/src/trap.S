.altmacro

.macro save_xn n
    sd x\n, \n*8(tp)
.endm

.macro load_xn n
    ld x\n, \n*8(tp)
.endm

    .global __stvec
    .global __restore_u_ctx
    /* The lowest two bits are always zero and are overridden
     * by mode.
     */
    .align 2
__stvec:
    csrrw tp, sscratch, tp

    /* If tp == 0, the trap is caused by kernel code and will
     * be handled by k_trap_handler.
     *
     * Otherwise, the trap is caused by user code and will be
     * handled by u_trap_handler. In this case, tp holds the
     * address of the trap context, where we save our context,
     * which is also the current kernel sp.
     */
    beqz tp, __save_k_ctx

__save_u_ctx:
    .set n, 0
    .rept 32
        save_xn %n
        .set n, n+1
    .endr
    csrrw x5, sscratch, x0
    sd x5, 4*8(tp)
    csrr x5, sstatus
    sd x5, 32*8(tp)
    csrr x5, sepc
    sd x5, 33*8(tp)
    mv sp, tp
    mv a0, tp
    call u_trap_handler

__restore_u_ctx:
    csrw sscratch, tp
    ld x5, 32*8(tp)
    csrw sstatus, x5
    ld x5, 33*8(tp)
    csrw sepc, x5
    .set n, 5
    .rept 27
        load_xn %n
        .set n, n+1
    .endr
    load_xn 1
    load_xn 2
    load_xn 3
    load_xn 4
    sret

__save_k_ctx:
    addi tp, sp, -35*8
    .set n, 0
    .rept 32
        save_xn %n
        .set n, n+1
    .endr
    csrrw x5, sscratch, x0
    sd x5, 4*8(tp)
    csrr x5, sstatus
    sd x5, 32*8(tp)
    csrr x5, sepc
    sd x5, 33*8(tp)
    sd x0, 34*8(tp)
    mv sp, tp
    mv a0, tp
    mv tp, x0
    call k_trap_handler

__restore_k_ctx:
    csrw sscratch, x0
    mv tp, sp
    ld x5, 32*8(tp)
    csrw sstatus, x5
    ld x5, 33*8(tp)
    csrw sepc, x5
    .set n, 5
    .rept 27
        load_xn %n
        .set n, n+1
    .endr
    load_xn 1
    load_xn 2
    load_xn 3
    load_xn 4
    sret