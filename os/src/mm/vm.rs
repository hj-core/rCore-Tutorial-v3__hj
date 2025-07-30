extern crate alloc;

use core::arch::asm;
use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::vec::Vec;
use xmas_elf::{ElfFile, program};

use crate::mm::page_alloc::{Page, alloc_page, alloc_zeroed_page};
use crate::mm::sv39::{PTE, PgtError, RootPgt};
use crate::mm::{
    KERNEL_VA_OFFSET, LARGE_PAGE_SIZE_BYTES, MEM_SIZE_BYTES, MEM_START_PA, PAGE_SIZE_BYTES,
    PAGE_SIZE_ORDER, PPN, QEMU_VIRT_MMIO, USER_SPACE_END, VPN, bss_end, bss_start, data_end,
    data_start, get_pa_from_va, get_pa_mut_ptr, get_va_from_pa, kernel_end, kernel_stack_end,
    kernel_stack_start, rodata_end, rodata_start, text_end, text_start,
};

const ALL_PERMISSION_FLAGS: usize = PERMISSION_R | PERMISSION_W | PERMISSION_X | PERMISSION_U;
const PERMISSION_R: usize = PTE::FLAG_R;
const PERMISSION_W: usize = PTE::FLAG_W;
const PERMISSION_X: usize = PTE::FLAG_X;
const PERMISSION_U: usize = PTE::FLAG_U;

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
    user_stack_end: usize,
    kernel_stack_end: usize,
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
            user_stack_end: 0,
            kernel_stack_end: 0,
        };
        result.map_user_elf(elf_bytes, MapType::Anonymous)?;
        result.map_user_stack()?;
        result.add_kernel_stack_area()?;
        Ok(result)
    }

    /// Maps the `elf_bytes`, creating the necessary [VMArea]s
    /// and [PTE]s.
    fn map_user_elf(
        self: &mut VMSpace,
        elf_bytes: &[u8],
        map_type: MapType,
    ) -> Result<(), VMError> {
        let elf = ElfFile::new(elf_bytes).map_err(|msg| VMError::ElfError(msg))?;

        for ph in elf.program_iter() {
            let ph_type = ph.get_type().map_err(|msg| VMError::ElfError(msg))?;
            if ph_type != program::Type::Load {
                continue;
            }

            let align = ph.align() as usize;
            if PAGE_SIZE_BYTES % align != 0 {
                return Err(VMError::AlignDataFailed(align));
            }

            let area = Self::create_area_from_ph(&ph, map_type)?;
            let start_vpn = area.start_vpn;
            let end_vpn = area.end_vpn;
            self.areas.push(area);

            let area_id = self.find_area(start_vpn)?;
            let mut file_start = ph.offset() as usize;
            let file_end = file_start + ph.file_size() as usize;

            for v in start_vpn.0..end_vpn.0 {
                let data = if file_start < file_end {
                    let bytes =
                        &elf_bytes[file_start..(file_start + PAGE_SIZE_BYTES).min(file_end)];
                    file_start += PAGE_SIZE_BYTES;

                    Some(bytes)
                } else {
                    None
                };
                self.map(VPN(v), area_id, data)?;
            }
        }

        self.entry_addr = elf.header.pt2.entry_point() as usize;
        Ok(())
    }

    fn create_area_from_ph(
        ph: &program::ProgramHeader,
        map_type: MapType,
    ) -> Result<VMArea, VMError> {
        let va_start = ph.virtual_addr() as usize;
        let mem_size = ph.mem_size() as usize;

        let start_vpn = VPN::from_va(va_start);
        let end_vpn = VPN::from_va(va_start + mem_size + PAGE_SIZE_BYTES - 1);
        let permissions = Self::get_permissions_from_ph_flags(ph.flags());

        Ok(VMArea::new(start_vpn, end_vpn, map_type, permissions))
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

    /// Returns an identifier to the [VMArea] containing the `vpn`,
    /// Or a [VMError] if such area does not exist.
    fn find_area(&self, vpn: VPN) -> Result<usize, VMError> {
        self.areas
            .iter()
            .rposition(|area| area.start_vpn <= vpn && vpn < area.end_vpn)
            .ok_or(VMError::NoAreaContainVpn(vpn))
    }

    fn get_area(&self, area_id: usize) -> Result<&VMArea, VMError> {
        self.areas
            .get(area_id)
            .ok_or(VMError::InvalidAreaId(area_id))
    }

    fn get_area_mut(&mut self, area_id: usize) -> Result<&mut VMArea, VMError> {
        self.areas
            .get_mut(area_id)
            .ok_or(VMError::InvalidAreaId(area_id))
    }

    /// Maps the `vpn` based on the [VMArea]. If `data` is
    /// not [None], it also copies the data to the mapped
    /// physical page.
    fn map(&mut self, vpn: VPN, area_id: usize, data: Option<&[u8]>) -> Result<(), VMError> {
        let area = self.get_area(area_id)?;
        if vpn < area.start_vpn || vpn >= area.end_vpn {
            return Err(VMError::VpnNotBelongArea(vpn, area_id));
        }

        let map_type = area.map_type;

        if data.is_some_and(|data| data.len() > PAGE_SIZE_BYTES) {
            return Err(VMError::DataExceedPage(vpn));
        }

        match map_type {
            MapType::Anonymous => unsafe { self.map_anonymous(vpn, area_id, data) },
            MapType::KernelVaOffset => panic!("Should already inherit from kernel space"),
        }
    }

    /// Maps the `vpn` to a newly allocated physical page.
    /// If `data` is not [None], it also copies the data to
    /// the mapped physical page.
    ///
    /// # Safety
    ///
    /// - `vpn` should within the [VMArea] that has the `area_id`.
    unsafe fn map_anonymous(
        &mut self,
        vpn: VPN,
        area_id: usize,
        data: Option<&[u8]>,
    ) -> Result<(), VMError> {
        let page = alloc_zeroed_page().ok_or(VMError::AcquirePageFailed)?;
        let ppn = page.get_ppn();

        let area = self.get_area_mut(area_id)?;
        let pte_flags = to_pte_flags(area.permissions)?;

        area.pages.push(page);
        self.root_pgt
            .map_create(vpn, ppn, pte_flags)
            .map_err(|pgt_err| VMError::PgtError(vpn, pgt_err))?;

        if let Some(data) = data {
            unsafe { Self::copy_data(ppn.get_pa(), data) }
        }
        Ok(())
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

    /// Maps a user stack of two pages right below the [USER_SPACE_END].
    fn map_user_stack(&mut self) -> Result<(), VMError> {
        let end_vpn = VPN::from_va(USER_SPACE_END);
        let start_vpn = VPN::from_va(USER_SPACE_END - 2 * PAGE_SIZE_BYTES);
        let area = VMArea::new(
            start_vpn,
            end_vpn,
            MapType::Anonymous,
            PERMISSION_R | PERMISSION_W | PERMISSION_U,
        );

        self.areas.push(area);
        let area_id = self.find_area(start_vpn)?;
        for v in start_vpn.0..end_vpn.0 {
            self.map(VPN(v), area_id, None)?;
        }

        self.user_stack_end = USER_SPACE_END;
        Ok(())
    }

    /// Assigns a page in the kernel memory range to be
    /// the kernel stack for this user space.
    fn add_kernel_stack_area(&mut self) -> Result<(), VMError> {
        let page = alloc_page().ok_or(VMError::AcquirePageFailed)?;
        let ppn = page.get_ppn();
        let pa = ppn.get_pa();
        let start_vpn = VPN::from_va(pa + KERNEL_VA_OFFSET);
        let end_vpn = VPN::from_va(pa + KERNEL_VA_OFFSET + PAGE_SIZE_BYTES);
        let permissions = PERMISSION_R | PERMISSION_W;
        let mut area = VMArea::new(start_vpn, end_vpn, MapType::KernelVaOffset, permissions);
        area.pages.push(page);
        self.areas.push(area);

        self.kernel_stack_end = end_vpn.get_va();
        Ok(())
    }

    pub(crate) fn get_satp(&self) -> usize {
        self.root_pgt.get_satp()
    }

    pub(crate) fn get_entry_addr(&self) -> usize {
        self.entry_addr
    }

    pub(crate) fn get_user_stack_end(&self) -> usize {
        self.user_stack_end
    }

    pub(crate) fn get_kernel_stack_end(&self) -> usize {
        self.kernel_stack_end
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
    pages: Vec<Page>,
}

impl VMArea {
    pub(crate) fn new(start_vpn: VPN, end_vpn: VPN, map_type: MapType, permissions: usize) -> Self {
        Self {
            start_vpn,
            end_vpn,
            map_type,
            permissions,
            pages: Vec::new(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum MapType {
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
    PgtError(VPN, PgtError),
    NoAreaContainVpn(VPN),
    VpnNotBelongArea(VPN, usize),
    InvalidAreaId(usize),
    InvalidPermissions(usize),
    ElfError(&'static str),
    DataExceedPage(VPN),
    AlignDataFailed(usize),
    AcquirePageFailed,
}
