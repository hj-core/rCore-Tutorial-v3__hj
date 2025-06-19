extern crate alloc;

use alloc::vec::Vec;
use lazy_static::lazy_static;

use crate::mm::page_alloc::{
    PAGE_SIZE_BYTES, PAGE_SIZE_ORDER, PHYS_MEM_BYTES, PHYS_MEM_START, PPN, Page,
};
use crate::mm::{
    bss_end, bss_start, data_end, data_start, get_kernel_end, rodata_end, rodata_start, text_end,
    text_start,
};
use crate::println;
use crate::sync::spin::SpinLock;

lazy_static! {
    static ref KERNEL_SPACE: SpinLock<VMSpace> = SpinLock::new(create_kernel_space());
}

/// Creates a VMSpace that matches the layout of kernel.
///
/// Currently, it contains a null root_pgt and no mappings, which
/// will be fixed when the page table abstraction is implemented.
fn create_kernel_space() -> VMSpace {
    let mut result = VMSpace {
        root_pgt: PPN(0),
        areas: Vec::new(),
    };
    push_kernel_text_area(&mut result);
    push_kernel_rodata_area(&mut result);
    push_kernel_data_area(&mut result);
    push_kernel_bss_area(&mut result);
    push_kernel_memory_area(&mut result);

    result
}

fn push_kernel_text_area(kernel_space: &mut VMSpace) {
    let area = VMArea {
        start_vpn: VPN::from_addr(text_start as usize),
        end_vpn: VPN::from_addr(text_end as usize),
        map_type: MapType::Identical,
        permissions: PERMISSION_R | PERMISSION_X,
        allocated_pages: Vec::new(),
    };
    kernel_space.push_area(area);
}

fn push_kernel_rodata_area(kernel_space: &mut VMSpace) {
    let area = VMArea {
        start_vpn: VPN::from_addr(rodata_start as usize),
        end_vpn: VPN::from_addr(rodata_end as usize),
        map_type: MapType::Identical,
        permissions: PERMISSION_R,
        allocated_pages: Vec::new(),
    };
    kernel_space.push_area(area);
}

fn push_kernel_data_area(kernel_space: &mut VMSpace) {
    let area = VMArea {
        start_vpn: VPN::from_addr(data_start as usize),
        end_vpn: VPN::from_addr(data_end as usize),
        map_type: MapType::Identical,
        permissions: PERMISSION_R | PERMISSION_W,
        allocated_pages: Vec::new(),
    };
    kernel_space.push_area(area);
}

fn push_kernel_bss_area(kernel_space: &mut VMSpace) {
    let area = VMArea {
        start_vpn: VPN::from_addr(bss_start as usize),
        end_vpn: VPN::from_addr(bss_end as usize),
        map_type: MapType::Identical,
        permissions: PERMISSION_R | PERMISSION_W,
        allocated_pages: Vec::new(),
    };
    kernel_space.push_area(area);
}

fn push_kernel_memory_area(kernel_space: &mut VMSpace) {
    let area = VMArea {
        start_vpn: compute_kernel_memory_start_vpn(),
        end_vpn: compute_kernel_memory_end_vpn(),
        map_type: MapType::Identical,
        permissions: PERMISSION_R | PERMISSION_W,
        allocated_pages: Vec::new(),
    };
    kernel_space.push_area(area);
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
    root_pgt: PPN,
    areas: Vec<VMArea>,
}

impl VMSpace {
    fn push_area(&mut self, area: VMArea) {
        self.areas.push(area);
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
struct VPN(usize);

impl VPN {
    fn from_addr(addr: usize) -> VPN {
        VPN(addr >> PAGE_SIZE_ORDER)
    }
}

#[derive(Debug)]
enum MapType {
    Identical,
}

const PERMISSION_R: usize = 1 << 1;
const PERMISSION_W: usize = 1 << 2;
const PERMISSION_X: usize = 1 << 3;
