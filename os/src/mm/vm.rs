extern crate alloc;

use alloc::vec::Vec;
use core::slice;

use lazy_static::lazy_static;
use xmas_elf::{
    ElfFile,
    program::{self, Flags},
};

use crate::mm::{
    QEMU_VIRT_MMIO, boot_stack_end, boot_stack_start, bss_end, bss_start, data_end, data_start,
    get_kernel_end,
    page_alloc::{PAGE_SIZE_BYTES, PAGE_SIZE_ORDER, PHYS_MEM_BYTES, PHYS_MEM_START},
    rodata_end, rodata_start,
    sv39::{PTE, PgtError, RootPgt},
    task_kernel_stacks_end, task_kernel_stacks_start, task_user_stacks_end, task_user_stacks_start,
    text_end, text_start, text_trap_end, text_trap_start,
};
use crate::println;
use crate::sync::spin::SpinLock;

lazy_static! {
    static ref KERNEL_SPACE: SpinLock<VMSpace> = SpinLock::new(create_kernel_space());
}

pub(crate) const PERMISSION_R: usize = PTE::FLAG_R;
pub(crate) const PERMISSION_W: usize = PTE::FLAG_W;
pub(crate) const PERMISSION_X: usize = PTE::FLAG_X;
pub(crate) const PERMISSION_U: usize = PTE::FLAG_U;
const PERMISSION_ALL_FLAGS: usize = PERMISSION_R | PERMISSION_W | PERMISSION_X | PERMISSION_U;

pub(crate) fn get_kernel_satp() -> usize {
    KERNEL_SPACE.lock().get_satp()
}

/// Creates a [VMSpace] that matches the layout of the kernel.
///
/// It eagerly propagates the page table entries according to its
/// [VMArea]s.
///
/// # Panic
/// * If it fails to create the root page table.
/// * If it fails to push an area.
fn create_kernel_space() -> VMSpace {
    let mut result = VMSpace::new().expect("Failed to create empty kernel space");

    push_qemu_mmio_areas(&mut result);
    push_kernel_text_area(&mut result);
    push_kernel_rodata_area(&mut result);
    push_kernel_data_area(&mut result);
    push_kernel_boot_stack_area(&mut result);
    push_kernel_task_kernel_stacks_area(&mut result);
    push_kernel_task_user_stacks_area(&mut result);
    push_kernel_bss_area(&mut result);
    push_kernel_memory_area(&mut result);

    result
}

fn push_qemu_mmio_areas(kernel_space: &mut VMSpace) {
    QEMU_VIRT_MMIO
        .iter()
        .map(|&(base, size)| {
            VMArea::new(
                VPN::from_addr(base),
                VPN::from_addr(base + size + PAGE_SIZE_BYTES - 1),
                MapType::Identical,
                PERMISSION_R | PERMISSION_W,
            )
        })
        .for_each(|area| {
            kernel_space
                .push_area(area, true)
                .expect("Failed to map qemu mmio area");
        });
}

fn push_kernel_text_area(kernel_space: &mut VMSpace) {
    let area = VMArea::new(
        VPN::from_addr(text_start as usize),
        VPN::from_addr(text_end as usize),
        MapType::Identical,
        PERMISSION_R | PERMISSION_X,
    );

    kernel_space
        .push_area(area, true)
        .expect("Failed to map kernel text area");
}

fn push_kernel_rodata_area(kernel_space: &mut VMSpace) {
    let area = VMArea::new(
        VPN::from_addr(rodata_start as usize),
        VPN::from_addr(rodata_end as usize),
        MapType::Identical,
        PERMISSION_R,
    );

    kernel_space
        .push_area(area, true)
        .expect("Failed to map kernel rodata area");
}

fn push_kernel_data_area(kernel_space: &mut VMSpace) {
    let area = VMArea::new(
        VPN::from_addr(data_start as usize),
        VPN::from_addr(data_end as usize),
        MapType::Identical,
        PERMISSION_R | PERMISSION_W,
    );

    kernel_space
        .push_area(area, true)
        .expect("Failed to map kernel data area");
}

fn push_kernel_boot_stack_area(kernel_space: &mut VMSpace) {
    let area = VMArea::new(
        VPN::from_addr(boot_stack_start as usize),
        VPN::from_addr(boot_stack_end as usize),
        MapType::Identical,
        PERMISSION_R | PERMISSION_W,
    );

    kernel_space
        .push_area(area, true)
        .expect("Failed to map kernel boot stack area");
}

fn push_kernel_task_kernel_stacks_area(kernel_space: &mut VMSpace) {
    let area = VMArea::new(
        VPN::from_addr(task_kernel_stacks_start as usize),
        VPN::from_addr(task_kernel_stacks_end as usize),
        MapType::Identical,
        PERMISSION_R | PERMISSION_W,
    );

    kernel_space
        .push_area(area, true)
        .expect("Failed to map kernel task kernel stacks area");
}

fn push_kernel_task_user_stacks_area(kernel_space: &mut VMSpace) {
    let area = VMArea::new(
        VPN::from_addr(task_user_stacks_start as usize),
        VPN::from_addr(task_user_stacks_end as usize),
        MapType::Identical,
        PERMISSION_R | PERMISSION_W,
    );

    kernel_space
        .push_area(area, true)
        .expect("Failed to map kernel task user stacks area");
}

fn push_kernel_bss_area(kernel_space: &mut VMSpace) {
    let area = VMArea::new(
        VPN::from_addr(bss_start as usize),
        VPN::from_addr(bss_end as usize),
        MapType::Identical,
        PERMISSION_R | PERMISSION_W,
    );

    kernel_space
        .push_area(area, true)
        .expect("Failed to map kernel bss area");
}

fn push_kernel_memory_area(kernel_space: &mut VMSpace) {
    let area = VMArea::new(
        compute_kernel_memory_start_vpn(),
        compute_kernel_memory_end_vpn(),
        MapType::Identical,
        PERMISSION_R | PERMISSION_W,
    );

    kernel_space
        .push_area(area, true)
        .expect("Failed to map kernel memory area");
}

fn compute_kernel_memory_start_vpn() -> VPN {
    let kernel_end = get_kernel_end();
    if kernel_end & (PAGE_SIZE_BYTES - 1) == 0 {
        VPN::from_addr(kernel_end)
    } else {
        VPN::from_addr(kernel_end + PAGE_SIZE_BYTES)
    }
}

/// Returns the exclusive end [VPN] of kernel memory [VMArea].
fn compute_kernel_memory_end_vpn() -> VPN {
    assert_eq!(
        PHYS_MEM_START & (PAGE_SIZE_BYTES - 1),
        0,
        "The algorithm assumes PYHS_MEM_START is page-aligned"
    );
    assert_eq!(
        PHYS_MEM_BYTES & (PAGE_SIZE_BYTES - 1),
        0,
        "The algorithm assumes PYHS_MEM_BYTES is page-aligned"
    );

    VPN::from_addr(PHYS_MEM_START + PHYS_MEM_BYTES)
}

#[allow(dead_code)]
pub(super) fn print_kernel_space() {
    println!("KERNEL_SPACE: {:#0x?}", KERNEL_SPACE.lock());
}

/// Loads the content of `elf_bytes` into `space`, creating the
/// necessary [VMArea]s and [PTE]s.
pub(crate) fn load_elf(
    space: &mut VMSpace,
    elf_bytes: &[u8],
    is_user: bool,
    map_type: MapType,
) -> Result<bool, VMError> {
    let elf = ElfFile::new(elf_bytes).map_err(|msg| VMError::ParseElfFailed(msg))?;

    for ph in elf.program_iter() {
        let header_type = ph.get_type().map_err(|msg| VMError::ParseElfFailed(msg))?;
        if header_type != program::Type::Load {
            continue;
        }

        let align = ph.align() as usize;
        if PAGE_SIZE_BYTES % align != 0 {
            return Err(VMError::DataNotPageAligned(align));
        }

        let area = create_area_from_ph(&ph, is_user, map_type)?;
        let start_vpn = area.start_vpn;
        let end_vpn = area.end_vpn;
        space.push_area(area, false)?;

        let area_id = space.find_area(start_vpn)?;
        let mut file_start = ph.offset() as usize;
        let file_end = file_start + ph.file_size() as usize;

        for v in start_vpn.0..end_vpn.0 {
            let data = if file_start < file_end {
                let bytes = &elf_bytes[file_start..(file_start + PAGE_SIZE_BYTES).min(file_end)];
                file_start += PAGE_SIZE_BYTES;

                Some(bytes)
            } else {
                None
            };
            space.map(VPN(v), area_id, data)?;
        }
    }
    Ok(true)
}

fn create_area_from_ph(
    ph: &program::ProgramHeader,
    is_user: bool,
    map_type: MapType,
) -> Result<VMArea, VMError> {
    let va_start = ph.virtual_addr() as usize;
    let mem_size = ph.mem_size() as usize;

    let start_vpn = VPN::from_addr(va_start);
    let end_vpn = VPN::from_addr(va_start + mem_size + PAGE_SIZE_BYTES - 1);
    let permissions = get_permissions_from_ph_flags(ph.flags(), is_user);

    Ok(VMArea::new(start_vpn, end_vpn, map_type, permissions))
}

fn get_permissions_from_ph_flags(flags: Flags, is_user: bool) -> usize {
    let mut result = if is_user { PERMISSION_U } else { 0 };

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

/// Pushes and eagerly maps the trap area.
pub(crate) fn push_trap_area(space: &mut VMSpace) -> Result<bool, VMError> {
    let area = VMArea::new(
        VPN::from_addr(text_trap_start as usize),
        VPN::from_addr(text_trap_end as usize),
        MapType::Identical,
        PERMISSION_X,
    );

    space.push_area(area, true)
}

/// A collection of related [VMArea]s that are controlled by
/// the same root page table.
#[derive(Debug)]
pub(crate) struct VMSpace {
    root_pgt: RootPgt,
    areas: Vec<VMArea>,
}

impl VMSpace {
    /// Returns a new [VMSpace] with a [RootPgt] and no areas.
    pub(crate) fn new() -> Result<Self, VMError> {
        let root_pgt = RootPgt::new().map_err(VMError::CreateRootPgtFailed)?;
        let areas = Vec::new();
        Ok(Self { root_pgt, areas })
    }

    pub(crate) fn get_satp(&self) -> usize {
        self.root_pgt.get_satp()
    }

    /// Push the `area` to this [VMSpace]. If `eager_mapping` is true,
    /// it also propagates the page tables and entries according to the
    /// `area`.
    ///
    /// If the mapping fails, it returns a [VMError::MappingError], which
    /// contains the [VPN] causing the failure and the corresponding [PgtError].
    /// However, the `area` would have already been added to `areas`, and
    /// the propagated page tables and entries are not rolled back.
    pub(crate) fn push_area(&mut self, area: VMArea, eager_mapping: bool) -> Result<bool, VMError> {
        if !eager_mapping {
            self.areas.push(area);
            return Ok(true);
        }

        let start_vpn = area.start_vpn;
        let end_vpn = area.end_vpn;
        self.areas.push(area);

        let area_id = self.find_area(start_vpn)?;
        for v in start_vpn.0..end_vpn.0 {
            self.map(VPN(v), area_id, None)?;
        }
        Ok(true)
    }

    /// Returns an identifier to the [VMArea] containing the `vpn`,
    /// Or a [VMError] if such area does not exist.
    fn find_area(&self, vpn: VPN) -> Result<usize, VMError> {
        self.areas
            .iter()
            .rposition(|area| area.start_vpn <= vpn && vpn < area.end_vpn)
            .ok_or(VMError::NoAreaForVpn(vpn))
    }

    fn get_area(&self, area_id: usize) -> Result<&VMArea, VMError> {
        self.areas
            .get(area_id)
            .ok_or(VMError::InvalidAreaId(area_id))
    }

    /// Maps the `vpn`. If `data` is not [None], it also copies the data
    /// to the mapped physical page.
    ///
    /// If the mapping fails, it returns the corresponding [VMError].
    fn map(&mut self, vpn: VPN, area_id: usize, data: Option<&[u8]>) -> Result<bool, VMError> {
        let area = self.get_area(area_id)?;
        if vpn < area.start_vpn || vpn >= area.end_vpn {
            return Err(VMError::AreaVpnMismatch(area_id, vpn));
        }

        let map_type = area.map_type;

        if data.is_some_and(|data| data.len() > PAGE_SIZE_BYTES) {
            return Err(VMError::DataExceedPage(vpn));
        }

        match map_type {
            MapType::Identical => self.map_identical(vpn, area_id, data),
        }
    }

    /// Maps the `vpn` to the physical page with the same page number.
    /// If `data` is not [None], it also copies the data to the mapped
    /// physical page.
    ///
    /// This function assumes the `vpn` belongs to the [VMArea] with
    /// `area_id`. If `data` is not [None], it also assumes the [VMArea]
    /// owns the mapped physical page.
    fn map_identical(
        &mut self,
        vpn: VPN,
        area_id: usize,
        data: Option<&[u8]>,
    ) -> Result<bool, VMError> {
        let va = vpn.get_virtual_addr();
        let pa = va;
        let pte_flags = Self::to_pte_flags(self.areas[area_id].permissions);

        self.root_pgt
            .map_create(va, pa, pte_flags)
            .map_err(|pgt_err| VMError::MappingError(vpn, pgt_err))?;

        if let Some(data) = data {
            unsafe { Self::copy_data(pa, data) };
        }
        Ok(true)
    }

    fn to_pte_flags(permissions: usize) -> usize {
        (permissions & PERMISSION_ALL_FLAGS) | PTE::FLAG_V
    }

    /// Copies `data` to the memory starting at `addr`.
    ///
    /// At most [PAGE_SIZE_BYTES] bytes are copied. If `data` is larger
    /// than a page, the extra data is ignored.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the address range starting from `addr`
    /// with a length of `data.len().min(PAGE_SIZE_BYTES)` is valid and
    /// writable.
    unsafe fn copy_data(addr: usize, data: &[u8]) {
        let dst =
            unsafe { slice::from_raw_parts_mut(addr as *mut u8, data.len().min(PAGE_SIZE_BYTES)) };
        dst.copy_from_slice(data);
    }
}

/// An abstraction over a range of virtual memory.
#[derive(Debug)]
pub(crate) struct VMArea {
    start_vpn: VPN,
    /// The exclusive end [VPN] of this [VMArea].
    end_vpn: VPN,
    map_type: MapType,
    permissions: usize,
}

impl VMArea {
    pub(crate) fn new(start_vpn: VPN, end_vpn: VPN, map_type: MapType, permissions: usize) -> Self {
        Self {
            start_vpn,
            end_vpn,
            map_type,
            permissions,
        }
    }
}

/// Virtual Page Number
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct VPN(pub(super) usize);

impl VPN {
    pub(crate) fn from_addr(addr: usize) -> VPN {
        VPN(addr >> PAGE_SIZE_ORDER)
    }

    fn get_virtual_addr(&self) -> usize {
        self.0 << PAGE_SIZE_ORDER
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum MapType {
    Identical,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum VMError {
    CreateRootPgtFailed(PgtError),
    NoAreaForVpn(VPN),
    AreaVpnMismatch(usize, VPN),
    InvalidAreaId(usize),
    InvalidPermissions(usize),
    MappingError(VPN, PgtError),
    DataExceedPage(VPN),
    ParseElfFailed(&'static str),
    DataNotPageAligned(usize),
}
