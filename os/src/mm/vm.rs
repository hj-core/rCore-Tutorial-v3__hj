extern crate alloc;

use alloc::collections::btree_map::BTreeMap;
use alloc::vec::Vec;
use core::arch::asm;
use core::sync::atomic::{AtomicUsize, Ordering};

use xmas_elf::{ElfFile, program};

use crate::mm::page_alloc::{Page, alloc_page, alloc_zeroed_page};
use crate::mm::sv39::{PTE, PgtError, RootPgt};
use crate::mm::{
    KERNEL_VA_OFFSET, LARGE_PAGE_SIZE_BYTES, MEM_SIZE_BYTES, MEM_START_PA, PAGE_SIZE_BYTES,
    PAGE_SIZE_ORDER, PPN, QEMU_VIRT_MMIO, USER_SPACE_END, USER_STACK_MAX_SIZE_BYTES, VPN, bss_end,
    bss_start, data_end, data_start, get_pa_from_va, get_pa_mut_ptr, get_va_from_pa, kernel_end,
    kernel_stack_end, kernel_stack_start, rodata_end, rodata_start, text_end, text_start,
};

const ALL_PERMISSION_FLAGS: usize = PERMISSION_R | PERMISSION_W | PERMISSION_X | PERMISSION_U;
pub(crate) const PERMISSION_R: usize = PTE::FLAG_R;
pub(crate) const PERMISSION_W: usize = PTE::FLAG_W;
pub(crate) const PERMISSION_X: usize = PTE::FLAG_X;
pub(crate) const PERMISSION_U: usize = PTE::FLAG_U;

fn to_pte_flags(permissions: usize) -> Result<usize, VMError> {
    if permissions & !ALL_PERMISSION_FLAGS != 0 {
        return Err(VMError::InvalidPermissions(permissions));
    }
    Ok((permissions & ALL_PERMISSION_FLAGS) | PTE::FLAG_V)
}

static KERNEL_SATP: AtomicUsize = AtomicUsize::new(0);

fn get_kernel_satp() -> usize {
    KERNEL_SATP.load(Ordering::Acquire)
}

fn get_kernel_satp_ppn() -> PPN {
    PPN::from_pa((get_kernel_satp() & 0xfff_ffff_ffff) << PAGE_SIZE_ORDER)
}

pub(crate) fn activate_kernel_space() {
    unsafe { asm!("csrw satp, {}", "sfence.vma", in(reg) get_kernel_satp()) }
}

/// Prepares the kernel page tables and updates the
/// [KERNEL_SATP].
pub(super) fn init_kernel_satp() -> Result<(), VMError> {
    let mut root_pgt = RootPgt::new().map_err(VMError::CreateRootPgtFailed)?;

    map_virt_mmio(&mut root_pgt)?;
    map_kernel_text(&mut root_pgt)?;
    map_kernel_stack(&mut root_pgt)?;
    map_kernel_rodata(&mut root_pgt)?;
    map_kernel_data(&mut root_pgt)?;
    map_kernel_bss(&mut root_pgt)?;
    map_phys_mem(&mut root_pgt)?;

    let satp = root_pgt.get_satp();
    KERNEL_SATP.store(satp, Ordering::Release);

    // We don't keep an instance of RootPgt for kernel space.
    // Here, we use its forget method to prevent the backing
    // pages from being recycled.
    let _ = unsafe { root_pgt.forget_self() };

    Ok(())
}

fn map_kernel_range(
    root_pgt: &mut RootPgt,
    start_va: usize,
    end_va: usize,
    permissions: usize,
) -> Result<(), VMError> {
    let pte_flags = to_pte_flags(permissions)?;
    (start_va..end_va)
        .step_by(PAGE_SIZE_BYTES)
        .map(|va| (VPN::from_va(va), PPN::from_pa(get_pa_from_va(va))))
        .try_for_each(|(vpn, ppn)| {
            root_pgt
                .map_create(vpn, ppn, pte_flags)
                .map_err(|pgt_err| VMError::PgtError(vpn, pgt_err))?;
            Ok(())
        })
}

fn map_virt_mmio(root_pgt: &mut RootPgt) -> Result<(), VMError> {
    QEMU_VIRT_MMIO
        .iter()
        .map(|&(start_pa, size)| (get_va_from_pa(start_pa), get_va_from_pa(start_pa + size)))
        .try_for_each(|(start_va, end_va)| {
            map_kernel_range(root_pgt, start_va, end_va, PERMISSION_R | PERMISSION_W)
        })
}

fn map_kernel_text(root_pgt: &mut RootPgt) -> Result<(), VMError> {
    map_kernel_range(
        root_pgt,
        text_start as usize,
        text_end as usize,
        PERMISSION_R | PERMISSION_X,
    )
}

fn map_kernel_stack(root_pgt: &mut RootPgt) -> Result<(), VMError> {
    map_kernel_range(
        root_pgt,
        kernel_stack_start as usize,
        kernel_stack_end as usize,
        PERMISSION_R | PERMISSION_W,
    )
}

fn map_kernel_rodata(root_pgt: &mut RootPgt) -> Result<(), VMError> {
    map_kernel_range(
        root_pgt,
        rodata_start as usize,
        rodata_end as usize,
        PERMISSION_R,
    )
}

fn map_kernel_data(root_pgt: &mut RootPgt) -> Result<(), VMError> {
    map_kernel_range(
        root_pgt,
        data_start as usize,
        data_end as usize,
        PERMISSION_R | PERMISSION_W,
    )
}

fn map_kernel_bss(root_pgt: &mut RootPgt) -> Result<(), VMError> {
    map_kernel_range(
        root_pgt,
        bss_start as usize,
        bss_end as usize,
        PERMISSION_R | PERMISSION_W,
    )
}

fn map_phys_mem(root_pgt: &mut RootPgt) -> Result<(), VMError> {
    let permissions = PERMISSION_R | PERMISSION_W;
    let start_va = compute_phy_mem_page_start(kernel_end as usize);
    let end_va = get_va_from_pa(MEM_START_PA + MEM_SIZE_BYTES);
    let large_start_va = compute_phy_mem_large_page_start(start_va);
    let large_end_va = compute_phy_mem_large_page_end(end_va);

    if large_end_va <= large_start_va {
        map_kernel_range(root_pgt, start_va, end_va, permissions)
    } else {
        map_kernel_range(root_pgt, start_va, large_start_va, permissions)?;
        map_kernel_range_large(root_pgt, large_start_va, large_end_va, permissions)?;
        map_kernel_range(root_pgt, large_end_va, end_va, permissions)
    }
}

fn compute_phy_mem_page_start(kernel_end: usize) -> usize {
    if kernel_end & (PAGE_SIZE_BYTES - 1) == 0 {
        kernel_end
    } else {
        kernel_end - (kernel_end & (PAGE_SIZE_BYTES - 1)) + PAGE_SIZE_BYTES
    }
}

fn compute_phy_mem_large_page_start(kernel_end: usize) -> usize {
    if kernel_end & (LARGE_PAGE_SIZE_BYTES - 1) == 0 {
        kernel_end
    } else {
        kernel_end - (kernel_end & (LARGE_PAGE_SIZE_BYTES - 1)) + LARGE_PAGE_SIZE_BYTES
    }
}

fn compute_phy_mem_large_page_end(phy_mem_end: usize) -> usize {
    if phy_mem_end & (LARGE_PAGE_SIZE_BYTES - 1) == 0 {
        phy_mem_end
    } else {
        phy_mem_end - (phy_mem_end & (LARGE_PAGE_SIZE_BYTES - 1))
    }
}

/// Maps the given range using large pages, i.e., 2 MiB
/// in size. The `start_va` and `end_va` must be aligned
/// to this size.
fn map_kernel_range_large(
    root_pgt: &mut RootPgt,
    start_va: usize,
    end_va: usize,
    permissions: usize,
) -> Result<(), VMError> {
    assert_eq!(
        0,
        start_va & (LARGE_PAGE_SIZE_BYTES - 1),
        "start_va must be aligned to the large page size"
    );
    assert_eq!(
        0,
        end_va & (LARGE_PAGE_SIZE_BYTES - 1),
        "end_va must be aligned to the large page size"
    );

    let pte_flags = to_pte_flags(permissions)?;
    (start_va..end_va)
        .step_by(LARGE_PAGE_SIZE_BYTES)
        .map(|va| (VPN::from_va(va), PPN::from_pa(get_pa_from_va(va))))
        .try_for_each(|(vpn, ppn)| {
            root_pgt
                .map_create_large(vpn, ppn, pte_flags)
                .map_err(|pgt_err| VMError::PgtError(vpn, pgt_err))?;
            Ok(())
        })
}

/// A collection of related [VMArea]s controlled by the
/// same [RootPgt]. This struct is used for user space
/// virtual memory management.
#[derive(Debug)]
pub(crate) struct VMSpace {
    root_pgt: RootPgt,
    areas: Vec<VMArea>,
    entry_addr: usize,
    /// The end of the task's user stack; i.e., the sp
    /// value when the stack is empty.
    u_stack_end: usize,
    /// The end of the task's kernel stack, i.e., the sp
    /// value when the stack is empty.
    k_stack_end: usize,
}

impl VMSpace {
    /// Returns a new user [VMSpace] with the `elf_bytes`
    /// mapped. Additionally, it inherits entries from the
    /// kernel's [RootPgt], and maps a user stack and a kernel
    /// stack.
    pub(crate) fn new_user(elf_bytes: &[u8]) -> Result<Self, VMError> {
        let entries = unsafe { RootPgt::get_ptes_mut(get_kernel_satp_ppn()) };
        let root_pgt =
            unsafe { RootPgt::new_copy(entries) }.map_err(VMError::CreateRootPgtFailed)?;
        let areas = Vec::new();

        let mut result = Self {
            root_pgt,
            areas,
            entry_addr: 0,
            u_stack_end: 0,
            k_stack_end: 0,
        };
        result.map_user_elf(elf_bytes)?;
        result.add_user_stack_area()?;
        result.add_kernel_stack_area()?;
        Ok(result)
    }

    /// Maps the `elf_bytes` in [MapType::Anonymous], creating
    /// the necessary [VMArea]s and [PTE]s.
    fn map_user_elf(self: &mut VMSpace, elf_bytes: &[u8]) -> Result<(), VMError> {
        let elf = ElfFile::new(elf_bytes).map_err(|msg| VMError::ElfError(msg))?;

        for ph in elf.program_iter() {
            let ph_type = ph.get_type().map_err(|msg| VMError::ElfError(msg))?;
            if ph_type != program::Type::Load {
                continue;
            }
            self.map_user_elf_segment(elf_bytes, &ph)?
        }

        self.entry_addr = elf.header.pt2.entry_point() as usize;
        Ok(())
    }

    fn map_user_elf_segment(
        self: &mut VMSpace,
        elf_bytes: &[u8],
        ph: &program::ProgramHeader,
    ) -> Result<(), VMError> {
        let align = ph.align() as usize;
        if PAGE_SIZE_BYTES % align != 0 {
            return Err(VMError::AlignDataFailed(align));
        }

        let mut area = Self::create_area_from_ph(&ph)?;
        let pte_flags = to_pte_flags(area.permissions)?;
        let mut data_start = ph.offset() as usize;
        let data_end = data_start + ph.file_size() as usize;

        for v in area.start_vpn.0..area.end_vpn.0 {
            let vpn = VPN(v);
            let page = alloc_zeroed_page().ok_or(VMError::AcquirePageFailed)?;
            let ppn = page.get_ppn();

            if data_start < data_end {
                unsafe {
                    Self::copy_data(
                        ppn.get_pa(),
                        &elf_bytes[data_start..(data_start + PAGE_SIZE_BYTES).min(data_end)],
                    );
                }
                data_start += PAGE_SIZE_BYTES;
            };

            self.root_pgt
                .map_create(vpn, ppn, pte_flags)
                .map_err(|pgt_err| VMError::PgtError(vpn, pgt_err))?;
            area.pages.insert(vpn, page);
        }

        self.areas.push(area);
        Ok(())
    }

    fn create_area_from_ph(ph: &program::ProgramHeader) -> Result<VMArea, VMError> {
        let va_start = ph.virtual_addr() as usize;
        let mem_size = ph.mem_size() as usize;

        let start_vpn = VPN::from_va(va_start);
        let end_vpn = VPN::from_va(va_start + mem_size + PAGE_SIZE_BYTES - 1);
        let permissions = Self::get_permissions_from_ph_flags(ph.flags());

        Ok(VMArea::new(
            start_vpn,
            end_vpn,
            MapType::Anonymous,
            permissions,
        ))
    }

    fn get_permissions_from_ph_flags(flags: program::Flags) -> usize {
        let mut result = PERMISSION_U;

        if flags.is_read() {
            result |= PERMISSION_R;
        }
        if flags.is_write() {
            result |= PERMISSION_W;
        }
        if flags.is_execute() {
            result |= PERMISSION_X;
        }

        result
    }

    /// Copies `data` to the memory starting at `pa`.
    ///
    /// At most [PAGE_SIZE_BYTES] bytes are copied. If `data`
    /// is larger than a page, the extra data is ignored.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the address range starting
    /// from `addr` with a length of `data.len().min(PAGE_SIZE_BYTES)`
    /// is valid and writable.
    unsafe fn copy_data(pa: usize, data: &[u8]) {
        let start = get_pa_mut_ptr(pa);
        for offset in 0..data.len().min(PAGE_SIZE_BYTES) {
            unsafe { start.add(offset).write_volatile(data[offset]) };
        }
    }

    /// Adds an area of [USER_STACK_MAX_SIZE_BYTES] that ends
    /// at [USER_SPACE_END] for the user stack. Lazily maps
    /// the pages, except the first page.
    fn add_user_stack_area(&mut self) -> Result<(), VMError> {
        let permissions = PERMISSION_R | PERMISSION_W | PERMISSION_U;
        let pte_flags = to_pte_flags(permissions)?;

        // Map the first page
        let page = alloc_zeroed_page().ok_or(VMError::AcquirePageFailed)?;
        let ppn = page.get_ppn();
        let vpn = VPN::from_va(USER_SPACE_END - PAGE_SIZE_BYTES);

        self.root_pgt
            .map_create(vpn, ppn, pte_flags)
            .map_err(|pgt_err| VMError::PgtError(vpn, pgt_err))?;

        // Create and push the area
        let end_vpn = VPN::from_va(USER_SPACE_END);
        let start_vpn = VPN::from_va(USER_SPACE_END - USER_STACK_MAX_SIZE_BYTES);
        let mut area = VMArea::new(start_vpn, end_vpn, MapType::Anonymous, permissions);
        area.pages.insert(vpn, page);
        self.areas.push(area);

        self.u_stack_end = USER_SPACE_END;
        Ok(())
    }

    /// Assigns a page in the kernel memory range to be the
    /// kernel stack for this user space.
    fn add_kernel_stack_area(&mut self) -> Result<(), VMError> {
        let page = alloc_page().ok_or(VMError::AcquirePageFailed)?;
        let ppn = page.get_ppn();
        let pa = ppn.get_pa();
        let start_vpn = VPN::from_va(pa + KERNEL_VA_OFFSET);
        let end_vpn = VPN::from_va(pa + KERNEL_VA_OFFSET + PAGE_SIZE_BYTES);
        let permissions = PERMISSION_R | PERMISSION_W;
        let mut area = VMArea::new(start_vpn, end_vpn, MapType::KernelVaOffset, permissions);
        area.pages.insert(start_vpn, page);
        self.areas.push(area);

        self.k_stack_end = end_vpn.get_va();
        Ok(())
    }

    pub(crate) fn get_satp(&self) -> usize {
        self.root_pgt.get_satp()
    }

    pub(crate) fn get_entry_addr(&self) -> usize {
        self.entry_addr
    }

    pub(crate) fn get_u_stack_end(&self) -> usize {
        self.u_stack_end
    }

    pub(crate) fn get_k_stack_end(&self) -> usize {
        self.k_stack_end
    }

    /// Maps the `vpn`, requesting at least the `min_permissions`.
    fn map(&mut self, vpn: VPN, min_permissions: usize) -> Result<(), VMError> {
        let area = self.find_area_mut(vpn)?;

        let permissions = area.permissions;
        if min_permissions & permissions != min_permissions {
            return Err(VMError::PermissionDenied(vpn, min_permissions));
        }

        let map_type = area.map_type;
        if !matches!(map_type, MapType::Anonymous) {
            panic!("Unexpected MapType {:?}.", map_type)
        }

        let page = alloc_zeroed_page().ok_or(VMError::AcquirePageFailed)?;
        let ppn = page.get_ppn();
        area.pages.insert(vpn, page);

        let pte_flags = to_pte_flags(area.permissions)?;
        let result = self
            .root_pgt
            .map_create(vpn, ppn, pte_flags)
            .map_err(|pgt_err| VMError::PgtError(vpn, pgt_err));

        if result.is_err() {
            let area = self.find_area_mut(vpn).unwrap();
            area.pages.remove(&vpn);
        }

        result
    }

    /// Returns a mutable reference to the [VMArea] containing
    /// the `vpn`, or a [VMError] if such area does not exist.
    fn find_area_mut(&mut self, vpn: VPN) -> Result<&mut VMArea, VMError> {
        self.areas
            .iter_mut()
            .find(|area| area.contain_vpn(vpn))
            .ok_or(VMError::NoAreaContainVpn(vpn))
    }

    /// Tries to map the `va` with the `min_permissions` into
    /// this [VMSpace]. The actual permissions follow the [VMArea]
    /// containing the `va`.
    pub(crate) fn map_fault_page(
        &mut self,
        va: usize,
        min_permissions: usize,
    ) -> Result<(), VMError> {
        self.map(VPN::from_va(va), min_permissions)
    }

    /// Adds a new [VMArea] according to the given properties,
    /// or returns the corresponding [VMError].
    pub(crate) fn add_new_area(
        &mut self,
        start_vpn: VPN,
        end_vpn: VPN,
        map_type: MapType,
        permissions: usize,
    ) -> Result<(), VMError> {
        if end_vpn <= start_vpn {
            return Err(VMError::EmptyArea(start_vpn, end_vpn));
        }

        if permissions & !ALL_PERMISSION_FLAGS != 0 {
            return Err(VMError::InvalidPermissions(permissions));
        }

        if self
            .areas
            .iter()
            .all(|area| area.end_vpn <= start_vpn || end_vpn <= area.start_vpn)
        {
            let area = VMArea::new(start_vpn, end_vpn, map_type, permissions);
            self.areas.push(area);
            Ok(())
        } else {
            Err(VMError::AreaOverlapping(start_vpn, end_vpn))
        }
    }
}

/// An abstraction over a range of virtual memory.
#[derive(Debug)]
struct VMArea {
    start_vpn: VPN,
    /// The exclusive end [VPN] of this [VMArea].
    end_vpn: VPN,
    map_type: MapType,
    permissions: usize,
    pages: BTreeMap<VPN, Page>,
}

impl VMArea {
    pub(crate) fn new(start_vpn: VPN, end_vpn: VPN, map_type: MapType, permissions: usize) -> Self {
        Self {
            start_vpn,
            end_vpn,
            map_type,
            permissions,
            pages: BTreeMap::new(),
        }
    }

    fn contain_vpn(&self, vpn: VPN) -> bool {
        self.start_vpn <= vpn && vpn < self.end_vpn
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum MapType {
    /// Maps the [VPN] to a newly allocated [PPN].
    Anonymous,
    /// Maps the [VPN] to the [PPN] located [KERNEL_VA_OFFSET]
    /// below it.
    KernelVaOffset,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum VMError {
    CreateRootPgtFailed(PgtError),
    /// (requested VPN, reported PgtError).
    PgtError(VPN, PgtError),
    /// (requested VPN).
    NoAreaContainVpn(VPN),
    /// (requested permissions).
    InvalidPermissions(usize),
    /// (request VPN, requested permissions).
    PermissionDenied(VPN, usize),
    /// (error message).
    ElfError(&'static str),
    /// (requested alignment).
    AlignDataFailed(usize),
    AcquirePageFailed,
    /// (start_vpn, end_vpn).
    AreaOverlapping(VPN, VPN),
    /// (start_vpn, end_vpn).
    EmptyArea(VPN, VPN),
}
