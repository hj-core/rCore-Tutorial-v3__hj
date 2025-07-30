extern crate alloc;

use core::mem::ManuallyDrop;

use alloc::vec::Vec;
use riscv::regs::satp::{self, Mode};

use crate::mm::page_alloc::{Page, alloc_page, alloc_zeroed_page};
use crate::mm::{LARGE_PAGE_SIZE_BYTES, PAGE_SIZE_ORDER, PPN, VPN, get_pa_mut_ptr};

const PTES_PER_TABLE: usize = 512;

/// Returns an array consisting of VPN_0, VPN_1, and VPN_2.
fn parse_vpn(vpn: VPN) -> [usize; 3] {
    let va = vpn.get_va();
    let vpn_0 = (va >> 12) & 0x1ff;
    let vpn_1 = (va >> 21) & 0x1ff;
    let vpn_2 = (va >> 30) & 0x1ff;

    [vpn_0, vpn_1, vpn_2]
}

/// Abstraction of the root page table for the Page-Based
/// 39-bit virtual memory system.
#[derive(Debug)]
pub(super) struct RootPgt {
    /// The physical page number (in Sv39) of the physical
    /// page backing this [RootPgt].
    ppn: PPN,
    /// The physical pages backing this [RootPgt] and its
    /// child tables.
    pages: Vec<Page>,
}

impl RootPgt {
    /// Creates a new [RootPgt] or returns the corresponding
    /// [PgtError].
    pub(super) fn new() -> Result<Self, PgtError> {
        let page = alloc_zeroed_page().ok_or(PgtError::AcquirePageFailed)?;
        let ppn = page.get_ppn();
        let pages = alloc::vec![page];

        Ok(Self { ppn, pages })
    }

    /// Create a new [RootPgt] initialized with the given
    /// `entries`.
    ///
    /// # Safety
    ///
    /// `entries` should be a valid page table.
    pub(super) unsafe fn new_copy(entries: &[PTE; PTES_PER_TABLE]) -> Result<Self, PgtError> {
        let page = alloc_page().ok_or(PgtError::AcquirePageFailed)?;
        let ppn = page.get_ppn();
        let pages = alloc::vec![page];

        unsafe {
            (get_pa_mut_ptr(ppn.get_pa()) as *mut [PTE; PTES_PER_TABLE])
                .as_mut()
                .unwrap()
                .copy_from_slice(entries);
        }

        Ok(Self { ppn, pages })
    }

    pub(super) fn get_satp(&self) -> usize {
        satp::compute_value(self.ppn.get_raw(), Mode::Sv39)
    }

    /// Returns a [PTE] array view of the physical page
    /// corresponding to the `ppn`.
    ///
    /// # Safety
    ///
    /// - The physical page corresponding to the `ppn`
    /// should be a valid page table.
    ///
    /// - The page table should remain valid after modifications,
    /// if any.
    pub(super) unsafe fn get_ptes_mut<'a>(ppn: PPN) -> &'a mut [PTE; PTES_PER_TABLE] {
        let ptr = get_pa_mut_ptr(ppn.get_pa()) as *mut [PTE; PTES_PER_TABLE];
        unsafe { ptr.as_mut().unwrap() }
    }

    /// Consumes the [RootPgt] but prevents the backing
    /// pages from being recycled. Returns the number of
    /// pages forgotten.
    ///
    /// This method is only for the kernel [RootPgt].
    pub(super) unsafe fn forget_self(self) -> usize {
        let result = self.pages.len();
        self.pages.into_iter().for_each(|page| {
            let _ = ManuallyDrop::new(page);
        });

        result
    }

    /// Maps `vpn` to `ppn` and constructs any necessary
    /// intermediate page tables.
    ///
    /// If an error occurs, it returns the corresponding
    /// [PgtError]. However, the newly created [PTE]s and
    /// acquired [Page]s are not rolled back.
    pub(super) fn map_create(
        &mut self,
        vpn: VPN,
        ppn: PPN,
        pte_flags: usize,
    ) -> Result<(), PgtError> {
        let parsed_vpn = parse_vpn(vpn);
        let leaf_pte = PTE::new(ppn, pte_flags)?;
        // SAFETY:
        // The RootPgt instance itself must point to a physical
        // page holding the corresponding page table.
        let mut table = unsafe { Self::get_ptes_mut(self.ppn) };

        // Walk to the leaf table
        for level in [2, 1] {
            if !table[parsed_vpn[level]].is_valid() {
                let page = alloc_zeroed_page().ok_or(PgtError::AcquirePageFailed)?;
                table[parsed_vpn[level]] = PTE::new(page.get_ppn(), PTE::FLAG_V)?;
                self.pages.push(page);
            }

            if table[parsed_vpn[level]].is_leaf() {
                return Err(PgtError::DoubleMapping(vpn, ppn));
            }

            // SAFETY:
            // The page table entry must be valid and point to a
            // physical page holding a page table; therefore, we
            // can cast a slice over it.
            table = unsafe { RootPgt::get_ptes_mut(table[parsed_vpn[level]].get_ppn()) };
        }

        // Update the leaf table
        if table[parsed_vpn[0]].is_valid() {
            return Err(PgtError::DoubleMapping(vpn, ppn));
        }
        table[parsed_vpn[0]] = leaf_pte;

        Ok(())
    }

    /// Maps the large page containing the `vpn` to the
    /// large page containing `ppn` and constructs any
    /// necessary intermediate page tables.
    ///
    /// If an error occurs, it returns the corresponding
    /// [PgtError]. However, the newly created [PTE]s and
    /// acquired [Page]s are not rolled back.
    ///
    /// A large page is [LARGE_PAGE_SIZE_BYTES] in size
    /// and is aligned to that size as well.
    pub(super) fn map_create_large(
        &mut self,
        vpn: VPN,
        ppn: PPN,
        pte_flags: usize,
    ) -> Result<(), PgtError> {
        let vpn = VPN::from_va(vpn.get_va() & !(LARGE_PAGE_SIZE_BYTES - 1));
        let ppn = PPN::from_pa(ppn.get_pa() & !(LARGE_PAGE_SIZE_BYTES - 1));

        let parsed_vpn = parse_vpn(vpn);
        let leaf_pte = PTE::new(ppn, pte_flags)?;

        // SAFETY:
        // The RootPgt instance itself must point to a physical
        // page holding the corresponding page table.
        let level2_table = unsafe { Self::get_ptes_mut(self.ppn) };
        if !level2_table[parsed_vpn[2]].is_valid() {
            let page = alloc_zeroed_page().ok_or(PgtError::AcquirePageFailed)?;
            level2_table[parsed_vpn[2]] = PTE::new(page.get_ppn(), PTE::FLAG_V)?;
            self.pages.push(page);
        }
        if level2_table[parsed_vpn[2]].is_leaf() {
            return Err(PgtError::DoubleMapping(vpn, ppn));
        }

        // SAFETY:
        // The existing page table entry or the one we newly
        // created must be valid and point to a physical page
        // holding a page table; therefore, we can cast a slice
        // over it.
        let level1_table = unsafe { RootPgt::get_ptes_mut(level2_table[parsed_vpn[2]].get_ppn()) };
        if level1_table[parsed_vpn[1]].is_leaf() {
            return Err(PgtError::DoubleMapping(vpn, ppn));
        }
        level1_table[parsed_vpn[1]] = leaf_pte;

        Ok(())
    }
}

// Page Table Entry
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(super) struct PTE(usize);

impl PTE {
    // The N and PBMT flags are excluded since Svnapot
    // extension is not implemented.
    const ALL_FLAGS: usize = 0x3ff;
    pub(super) const FLAG_V: usize = 1 << 0;
    pub(super) const FLAG_R: usize = 1 << 1;
    pub(super) const FLAG_W: usize = 1 << 2;
    pub(super) const FLAG_X: usize = 1 << 3;
    pub(super) const FLAG_U: usize = 1 << 4;

    /// Creates a [PTE] that points to the physical page
    /// containing the `pa`, or returns the corresponding
    /// [PgtError].
    ///
    /// Svnapot extension is not implemented.
    fn new(ppn: PPN, pte_flags: usize) -> Result<Self, PgtError> {
        if !Self::is_valid_flags(pte_flags) {
            return Err(PgtError::InvalidPteFlags(pte_flags));
        }
        let value = (ppn.get_raw() << 10) | pte_flags;
        Ok(Self(value))
    }

    fn is_valid_flags(pte_flags: usize) -> bool {
        if Self::has_unknown_flags_set(pte_flags)
            || Self::has_v_flag_clear(pte_flags)
            || Self::is_reserved_xwr_encodings(pte_flags)
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

    fn is_reserved_xwr_encodings(pte_flags: usize) -> bool {
        let xwr = (pte_flags & (Self::FLAG_X | Self::FLAG_W | Self::FLAG_R)) >> 1;
        xwr == 0b010 || xwr == 0b110
    }

    fn is_valid(&self) -> bool {
        self.0 & Self::FLAG_V != 0
    }

    fn get_ppn(&self) -> PPN {
        let ppn = (self.0 >> 10) & 0xfff_ffff_ffff;
        PPN::from_pa(ppn << PAGE_SIZE_ORDER)
    }

    fn is_leaf(&self) -> bool {
        self.is_valid() && self.0 & (Self::FLAG_X | Self::FLAG_W) != 0
    }
}

#[derive(Debug)]
pub(crate) enum PgtError {
    AcquirePageFailed,
    #[allow(dead_code)]
    InvalidPteFlags(usize),
    #[allow(dead_code)]
    /// Attempts to map a [VPN] that is already mapped.
    DoubleMapping(VPN, PPN),
}
