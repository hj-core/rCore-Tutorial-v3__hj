    .set PAGE_OFFSET, 0xffffffc000000000
    .set SATP_SV39_MODE, 8
    .set PTE_FLAG_RWXV, 0xf

    .section .text.entry
    .global _start
_start:
    la sp, kernel_stack_end
    call .L_activate_boot_pgt

    li t6, PAGE_OFFSET         # Adjust SP
    add sp, sp, t6
    la t0, rust_main           # Jump to VA(rust_main)
    add t0, t0, t6
    jalr x0, t0, 0

# Create and activate a temporary page table. It uses huge
# page mapping.
.L_activate_boot_pgt:
    # Clear the boot page table
Init1:
    la t0, boot_pgt
    li t1, 512
    slli t1, t1, 3
    add t1, t0, t1
Loop1:
    bge t0, t1, Done1
    sd zero, 0(t0)
    addi t0, t0, 8
    j Loop1
Done1:
    # Map the entry page identically, required to fetch the
    # instruction immediately after writing to satp.
    la t0, text_start          # t0: PA
    la t1, text_start          # t1: VPN2 of PA
    srli t1, t1, 30
    mv t2, t1                  # t2: PTE for the huge page containing PA
    slli t2, t2, 28            # 28=30-12+10
    xor t2, t2, PTE_FLAG_RWXV
    la t3, boot_pgt            # Save PTE
    slli t1, t1, 3
    add t3, t3, t1
    sd t2, 0(t3)
    # Map the high space kernel to the physical pages
Init2:
    li t6, 1                  # t6: Mask that PTE(PA)^Mask=PTE(VA)
    slli t6, t6, 8
    li t5, 1                  # t5: Huge page size
    slli t5, t5, 30
    la t0, boot_pgt           # t0: Addr(PTE to be write)
    li t1, 256
    slli t1, t1, 3
    add t0, t0, t1
    add t1, t1, t0            # t1: Addr(Last PTE)+8
    li t2, 0                  # t2: PA to be mapped
Loop2:
    bge t0, t1, Done2
    mv t3, t2                 # t3: PTE(VA) for current PA
    srli t3, t3, 12
    slli t3, t3, 10
    xor t3, t3, t6
    xor t3, t3, PTE_FLAG_RWXV
    sd t3, 0(t0)              # Save PTE
    addi t0, t0, 8
    add t2, t2, t5
    j Loop2
Done2:
    # Activate boot page table
    la t0, boot_pgt
    srli t0, t0, 12
    li t1, SATP_SV39_MODE
    slli t1, t1, 60
    xor t0, t0, t1
    csrw satp, t0
    sfence.vma
    ret
