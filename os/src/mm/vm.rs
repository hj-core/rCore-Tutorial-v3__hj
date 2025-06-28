extern crate alloc;

use core::arch::asm;

use alloc::vec::Vec;
use lazy_static::lazy_static;
use riscv::regs::satp::{self, Mode};

use crate::mm::page_alloc::{
    PAGE_SIZE_BYTES, PAGE_SIZE_ORDER, PHYS_MEM_BYTES, PHYS_MEM_START, Page,
};
use crate::mm::sv39::{PTE, PgtError, RootPgt};
use crate::mm::{
    QEMU_VIRT_MMIO, bss_end, bss_start, data_end, data_start, get_kernel_end, rodata_end,
    rodata_start, text_end, text_start,
};
use crate::println;
use crate::sync::spin::SpinLock;

lazy_static! {
    static ref KERNEL_SPACE: SpinLock<VMSpace> = SpinLock::new(create_kernel_space());
}

const PERMISSION_R: usize = PTE::FLAG_R;
const PERMISSION_W: usize = PTE::FLAG_W;
const PERMISSION_X: usize = PTE::FLAG_X;

pub(super) fn enable_satp() {
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
    let root_pgt = RootPgt::new().expect("Failed to create root page table for kernel space");

    let mut result = VMSpace {
        root_pgt,
        areas: Vec::new(),
    };
    push_qemu_mmio_areas(&mut result);
    push_kernel_text_area(&mut result);
    push_kernel_rodata_area(&mut result);
    push_kernel_data_area(&mut result);
    push_kernel_boot_stack_area(&mut result);
    push_kernel_bss_area(&mut result);
    push_kernel_memory_area(&mut result);

    result
}

fn push_qemu_mmio_areas(kernel_space: &mut VMSpace) {
    QEMU_VIRT_MMIO
        .iter()
        .map(|&(base, size)| VMArea {
            start_vpn: VPN::from_addr(base),
            end_vpn: VPN::from_addr(base + size.max(PAGE_SIZE_BYTES)),
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
        permissions: PERMISSION_R | PERMISSION_W,
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
    assert!(
        PHYS_MEM_BYTES & (PAGE_SIZE_BYTES - 1) == 0,
        "The algorithm assumes PYHS_MEM_BYTES is page-aligned"
    );

    VPN::from_addr(PHYS_MEM_START + PHYS_MEM_BYTES)
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
        let map_type = area.map_type;
        let permissions = area.permissions;

        self.areas.push(area);
        for v in start_vpn.0..end_vpn.0 {
            if let Err(e) = self.map(VPN(v), permissions, map_type) {
                return Err(e);
            }
        }
        Ok(true)
    }

    fn map(&mut self, vpn: VPN, permissions: usize, map_type: MapType) -> Result<bool, VMError> {
        match map_type {
            MapType::Identical => self.map_identical(vpn, permissions),
        }
    }

    fn map_identical(&mut self, vpn: VPN, permissions: usize) -> Result<bool, VMError> {
        let va = vpn.get_virtual_addr();
        let pte_flags = permissions | PTE::FLAG_V;
        self.root_pgt
            .map_create(va, va, pte_flags)
            .map_err(|pgt_err| VMError::MappingError(vpn, pgt_err))
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
    MappingError(VPN, PgtError),
}
