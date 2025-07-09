extern crate alloc;
use core::slice;

use alloc::vec::Vec;
use riscv::regs::satp::{self, Mode};

use crate::mm::page_alloc::{self, Page};

// Sv39 physical address is limited to 56 bits.
const MAX_PHYS_ADDR: usize = (1 << 56) - 1;
const PAGE_OFFSET_ORDER: usize = 12;
const PTES_PER_TABLE: usize = 512;

fn acquire_zeroed_page() -> Result<Page, PgtError> {
    page_alloc::alloc_zeroed_page().ok_or(PgtError::AcquirePageFailed)
}

/// Obtains a pointer to the physical address `pa`. The current
/// implementation assumes an identical mapping.
fn get_pa_mut_ptr(pa: usize) -> *mut u8 {
    pa as *mut u8
}

/// Returns whether the physical address complies with the Sv39 scheme.
fn is_valid_pa(pa: usize) -> bool {
    pa <= MAX_PHYS_ADDR
}

/// Returns whether the virtual address complies with the Sv39 scheme.
fn is_valid_va(va: usize) -> bool {
    // va must have bits 63-39 all equal to bit 38 (0-indexed) according
    // to the Sv39 scheme.
    let mask = 0xffff_ffc0_0000_0000;
    va & mask == 0 || va & mask == mask
}

/// Returns an array of VPN_2, VPN_1, VPN_0 and page_offset according
/// to the Sv39 scheme, or a [PgtError] if the `va` is invalid.
fn parse_virtual_addr(va: usize) -> Result<[usize; 4], PgtError> {
    if !is_valid_va(va) {
        return Err(PgtError::InvalidVirtualAddress);
    }

    let vpn_2 = (va >> 30) & 0x1ff;
    let vpn_1 = (va >> 21) & 0x1ff;
    let vpn_0 = (va >> 12) & 0x1ff;
    let page_offset = va & 0xfff;

    Ok([vpn_2, vpn_1, vpn_0, page_offset])
}

/// Abstraction of the root page table for the Page-Based 39-bit
/// virtual memory system.
///
/// # Invariants:
///
/// * Instances of [RootPgt] should only be created through the
/// [RootPgt::new] method.
///
/// * Any valid non-leaf [PTE] in a page table must point to a
/// physical page holding the [PTE]s of a page table.
#[derive(Debug)]
pub(super) struct RootPgt {
    /// The physical page number (in Sv39) of the physical page
    /// backing the [PTE]s of the [RootPgt].
    ppn: usize,
    /// The physical pages backing the [PTE]s of the [RootPgt]
    /// and its child tables.
    pages: Vec<Page>,
}

impl RootPgt {
    /// Creates a new [RootPgt] or returns the corresponding
    /// [PgtError].
    pub(super) fn new() -> Result<Self, PgtError> {
        let page = acquire_zeroed_page()?;

        let pa = page.get_physical_addr();
        if !is_valid_pa(pa) {
            return Err(PgtError::InvalidPhysicalAddress);
        }

        Ok(Self {
            ppn: pa >> PAGE_OFFSET_ORDER,
            pages: alloc::vec![page],
        })
    }

    pub(super) fn get_satp(&self) -> usize {
        satp::compute_value(self.ppn, Mode::Sv39)
    }

    /// Returns a slice over the physical page corresponding to the
    /// `ppn` in Sv39.
    ///
    /// # Safety:
    /// * The physical page corresponding to the `ppn` should indeed
    /// hold the [PTE]s of a page table.
    unsafe fn as_mut_slice_from_ppn<'a>(ppn: usize) -> &'a mut [PTE] {
        let pa = get_pa_mut_ptr(ppn << PAGE_OFFSET_ORDER);
        unsafe { slice::from_raw_parts_mut(pa as *mut PTE, PTES_PER_TABLE) }
    }

    /// Maps the virtual page containing the `va` to the physical page
    /// containing the `pa`, and constructs any necessary intermediate
    /// page tables.
    ///
    /// If an error occurrs, it returns the corresponding [PgtError].
    /// However, the newly created [PTE]s and acquired [Page]s are not
    /// rolled back.
    pub(super) fn map_create(
        &mut self,
        va: usize,
        pa: usize,
        pte_flags: usize,
    ) -> Result<bool, PgtError> {
        let va = parse_virtual_addr(va)?;
        let leaf_pte = PTE::new(pa, pte_flags)?;
        // SAFETY:
        // The RootPgt instance itself must point to a physical page holding
        // the corresponding page table.
        let mut table = unsafe { Self::as_mut_slice_from_ppn(self.ppn) };

        // Walk to the leaf table
        for step in 0..2 {
            if !table[va[step]].is_valid() {
                let page = acquire_zeroed_page()?;
                table[va[step]] = PTE::new(page.get_physical_addr(), PTE::FLAG_V)?;
                self.pages.push(page);
            }

            if table[va[step]].is_leaf() {
                return Err(PgtError::HugePageNotSupported);
            }

            // SAFETY:
            // The page table entry must be valid and point to a physical page
            // holding a page table; therefore, we can cast a slice over it.
            table = unsafe { RootPgt::as_mut_slice_from_ppn(table[va[step]].get_ppn()) };
        }

        // Update the leaf table
        if table[va[2]].is_valid() {
            return Err(PgtError::DoubleMapping);
        }
        table[va[2]] = leaf_pte;

        Ok(true)
    }
}

// Page Table Entry
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(super) struct PTE(usize);

impl PTE {
    // The N and PBMT flags are excluded since Svnapot extension is
    // not implemented.
    const ALL_FLAGS: usize = 0x3ff;
    pub(super) const FLAG_V: usize = 1 << 0;
    pub(super) const FLAG_R: usize = 1 << 1;
    pub(super) const FLAG_W: usize = 1 << 2;
    pub(super) const FLAG_X: usize = 1 << 3;
    pub(super) const FLAG_U: usize = 1 << 4;

    /// Creates a [PTE] that points to the physical page containing
    /// the `pa`, or returns the corresponding [PgtError].
    ///
    /// Svnapot extension is not implemented.
    fn new(pa: usize, pte_flags: usize) -> Result<Self, PgtError> {
        if !is_valid_pa(pa) {
            return Err(PgtError::InvalidPhysicalAddress);
        }

        if !Self::is_valid_flags(pte_flags) {
            return Err(PgtError::InvalidPteFlags);
        }

        let ppn = pa >> PAGE_OFFSET_ORDER;
        let value = (ppn << 10) | pte_flags;
        Ok(Self(value))
    }

    fn is_valid_flags(pte_flags: usize) -> bool {
        if Self::has_unknown_flags_set(pte_flags)
            || Self::has_v_flag_clear(pte_flags)
            || Self::is_reserved_xwr_flag_encodings(pte_flags)
        {
            return false;
        }

        true
    }

    fn has_unknown_flags_set(pte_flags: usize) -> bool {
        pte_flags & !Self::ALL_FLAGS != 0
    }

    fn has_v_flag_clear(pte_flags: usize) -> bool {
        pte_flags & Self::FLAG_V == 0
    }

    fn is_reserved_xwr_flag_encodings(pte_flags: usize) -> bool {
        let xwr = (pte_flags & (Self::FLAG_X | Self::FLAG_W | Self::FLAG_R)) >> 1;
        xwr == 0b010 || xwr == 0b110
    }

    fn is_valid(&self) -> bool {
        self.0 & Self::FLAG_V != 0
    }

    fn get_ppn(&self) -> usize {
        (self.0 >> 10) & 0xfff_ffff_ffff
    }

    fn is_leaf(&self) -> bool {
        self.is_valid() && self.0 & (Self::FLAG_X | Self::FLAG_W) != 0
    }
}

#[derive(Debug)]
pub(crate) enum PgtError {
    InvalidVirtualAddress,
    InvalidPhysicalAddress,
    InvalidPteFlags,
    AcquirePageFailed,
    HugePageNotSupported,
    DoubleMapping,
}
