extern crate alloc;

use alloc::vec::Vec;
use core::arch::asm;
use core::slice;

use lazy_static::lazy_static;
use riscv::regs::{
    satp::{self, Mode},
    sstatus,
};
use xmas_elf::{
    ElfFile,
    program::{self, Flags},
};

use crate::mm::page_alloc::{PAGE_SIZE_BYTES, PAGE_SIZE_ORDER, PHYS_MEM_START, Page};
use crate::mm::sv39::{PTE, PgtError, RootPgt};
use crate::mm::{
    QEMU_VIRT_MMIO, bss_end, bss_start, data_end, data_start, get_kernel_end, rodata_end,
    rodata_start, text_end, text_start,
};
use crate::sync::spin::SpinLock;
use crate::task::prelude::get_app_entry_ptr;
use crate::{
    println,
    task::prelude::{get_app_elf_bytes, get_total_apps},
};

lazy_static! {
    static ref KERNEL_SPACE: SpinLock<VMSpace> = SpinLock::new(create_kernel_space());
}

const PERMISSION_R: usize = PTE::FLAG_R;
const PERMISSION_W: usize = PTE::FLAG_W;
const PERMISSION_X: usize = PTE::FLAG_X;
const PERMISSION_U: usize = PTE::FLAG_U;
const PERMISSION_ALL_FLAGS: usize = PERMISSION_R | PERMISSION_W | PERMISSION_X | PERMISSION_U;

pub(super) fn enable_satp() {
    sstatus::set_sum_permit();

    let ppn = KERNEL_SPACE.lock().root_pgt.get_ppn();
    satp::enable(ppn, Mode::Sv39);
    unsafe { asm!("sfence.vma") };
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
    push_kernel_bss_area(&mut result);
    push_kernel_memory_area(&mut result);
    push_kernel_apps_areas(&mut result);

    result
}

fn push_qemu_mmio_areas(kernel_space: &mut VMSpace) {
    QEMU_VIRT_MMIO
        .iter()
        .map(|&(base, size)| VMArea {
            start_vpn: VPN::from_addr(base),
            end_vpn: VPN::from_addr(base + size + PAGE_SIZE_BYTES - 1),
            map_type: MapType::Identical,
            permissions: PERMISSION_R | PERMISSION_W,
            allocated_pages: Vec::new(),
        })
        .for_each(|area| {
            kernel_space
                .push_area(area, true)
                .expect("Failed to map qemu mmio area");
        });
}

fn push_kernel_text_area(kernel_space: &mut VMSpace) {
    let area = VMArea {
        start_vpn: VPN::from_addr(text_start as usize),
        end_vpn: VPN::from_addr(text_end as usize),
        map_type: MapType::Identical,
        permissions: PERMISSION_R | PERMISSION_X,
        allocated_pages: Vec::new(),
    };

    kernel_space
        .push_area(area, true)
        .expect("Failed to map kernel text area");
}

fn push_kernel_rodata_area(kernel_space: &mut VMSpace) {
    let area = VMArea {
        start_vpn: VPN::from_addr(rodata_start as usize),
        end_vpn: VPN::from_addr(rodata_end as usize),
        map_type: MapType::Identical,
        permissions: PERMISSION_R,
        allocated_pages: Vec::new(),
    };

    kernel_space
        .push_area(area, true)
        .expect("Failed to map kernel rodata area");
}

fn push_kernel_data_area(kernel_space: &mut VMSpace) {
    let area = VMArea {
        start_vpn: VPN::from_addr(data_start as usize),
        end_vpn: VPN::from_addr(data_end as usize),
        map_type: MapType::Identical,
        permissions: PERMISSION_R | PERMISSION_W,
        allocated_pages: Vec::new(),
    };

    kernel_space
        .push_area(area, true)
        .expect("Failed to map kernel data area");
}

fn push_kernel_boot_stack_area(kernel_space: &mut VMSpace) {
    let area = VMArea {
        start_vpn: VPN::from_addr(data_end as usize),
        end_vpn: VPN::from_addr(bss_start as usize),
        map_type: MapType::Identical,
        permissions: PERMISSION_R | PERMISSION_W,
        allocated_pages: Vec::new(),
    };

    kernel_space
        .push_area(area, true)
        .expect("Failed to map kernel boot stack area");
}

fn push_kernel_bss_area(kernel_space: &mut VMSpace) {
    let area = VMArea {
        start_vpn: VPN::from_addr(bss_start as usize),
        end_vpn: VPN::from_addr(bss_end as usize),
        map_type: MapType::Identical,
        // Requrie PERMISSION_U because we have User stacks in it
        permissions: PERMISSION_R | PERMISSION_W | PERMISSION_U,
        allocated_pages: Vec::new(),
    };

    kernel_space
        .push_area(area, true)
        .expect("Failed to map kernel bss area");
}

fn push_kernel_memory_area(kernel_space: &mut VMSpace) {
    let area = VMArea {
        start_vpn: compute_kernel_memory_start_vpn(),
        end_vpn: compute_kernel_memory_end_vpn(),
        map_type: MapType::Identical,
        permissions: PERMISSION_R | PERMISSION_W,
        allocated_pages: Vec::new(),
    };

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
    assert!(
        PHYS_MEM_START & (PAGE_SIZE_BYTES - 1) == 0,
        "The algorithm assumes PYHS_MEM_START is page-aligned"
    );

    VPN::from_addr(get_app_entry_ptr(0).addr())
}

fn push_kernel_apps_areas(kernel_space: &mut VMSpace) {
    for app_index in 0..get_total_apps() {
        let elf_bytes = get_app_elf_bytes(app_index);
        load_elf(kernel_space, elf_bytes, true, MapType::Identical).expect("Failed to load app");
    }
}

/// Creates [VMArea]s according to the layout specified in the
/// `elf_input` and pushes them into `space`.
fn load_elf(
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

    Ok(VMArea {
        start_vpn,
        end_vpn,
        map_type,
        permissions,
        allocated_pages: Vec::new(),
    })
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

#[allow(dead_code)]
pub(super) fn print_kernel_space() {
    println!("KERNEL_SPACE: {:#0x?}", KERNEL_SPACE.lock());
}

/// A collection of related [VMArea]s that are controlled by
/// the same root page table.
#[derive(Debug)]
struct VMSpace {
    root_pgt: RootPgt,
    areas: Vec<VMArea>,
}

impl VMSpace {
    /// Returns a new [VMSpace] with a [RootPgt] and no areas.
    fn new() -> Result<Self, VMError> {
        let root_pgt = RootPgt::new().map_err(VMError::CreateRootPgtFailed)?;
        let areas = Vec::new();
        Ok(Self { root_pgt, areas })
    }

    /// Push the `area` to this [VMSpace]. If `eager_mapping` is true,
    /// it also propagates the page tables and entries according to the
    /// `area`.
    ///
    /// If the mapping fails, it returns a [VMError::MappingError], which
    /// contains the [VPN] causing the failure and the corresponding [PgtError].
    /// However, the `area` would have already been added to `areas`, and
    /// the propagated page tables and entries are not rolled back.
    fn push_area(&mut self, area: VMArea, eager_mapping: bool) -> Result<bool, VMError> {
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
struct VMArea {
    start_vpn: VPN,
    /// The exclusive end [VPN] of this [VMArea].
    end_vpn: VPN,
    map_type: MapType,
    permissions: usize,
    allocated_pages: Vec<Page>,
}

/// Virtual Page Number
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct VPN(pub(super) usize);

impl VPN {
    fn from_addr(addr: usize) -> VPN {
        VPN(addr >> PAGE_SIZE_ORDER)
    }

    fn get_virtual_addr(&self) -> usize {
        self.0 << PAGE_SIZE_ORDER
    }
}

#[derive(Debug, Copy, Clone)]
enum MapType {
    Identical,
}

#[allow(dead_code)]
#[derive(Debug)]
enum VMError {
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
