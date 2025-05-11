.altmacro

.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm

.macro RESTORE_GP n
    ld x\n, \n*8(sp)
.endm

    .section .text
    .global __stvec
    .global __restore
    .align 2                  # The lowest two bits are always zero and are overridden by mode
    # Invariant:
    # The sscratch contains the kernel_sp when entering the trap.
__stvec:
    csrrw sp, sscratch, sp    # Swap the value of sp and sscratch
    addi sp, sp, -34*8        # Make room for the TrapContext in the kernel stack
    SAVE_GP 1                 # x2, i.e. sp, will be saved later
    SAVE_GP 3                 # x4, i.e. tp, is skipped because it is not used by applications
    .set n, 5                 # Saved x5 through x31
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr                     # We can use the x5 through x7 since they have been saved
    csrr x5, sstatus          # Save the sstatus, sepc and user_sp
    csrr x6, sepc
    csrr x7, sscratch
    sd x5, 32*8(sp)
    sd x6, 33*8(sp)
    sd x7, 2*8(sp)

    mv x10, sp                # Set the arugment for trap_handler, i.e., a pointer to the saved context
    call trap_handler

    # We reach this point either when we return from the trap_handler or when we directly call
    # __restore. The latter case occurs when we run a new app and use __restore to configure its
    # context. In both cases, we should pass the correct pointer to the saved TrapContext, i.e.,
    # the kernel_sp, as the argument to __restore.
__restore:
    mv sp, x10                # Restore the kernel_sp
    ld x5, 32*8(sp)           # Restore the sstatus, sepc and sscratch
    ld x6, 33*8(sp)
    ld x7, 2*8(sp)
    csrw sstatus, x5
    csrw sepc, x6
    csrw sscratch, x7
    RESTORE_GP 1              # Restore the saved registers
    RESTORE_GP 3
    .set n, 5
    .rept 27
        RESTORE_GP %n
        .set n, n+1
    .endr
    addi sp, sp, 34*8         # Pop the room for the TrapContext from the kernel stak
    csrrw sp, sscratch, sp    # Swap the value of sp and sscratch
    sret
