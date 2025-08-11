mod heap_alloc;
mod page_alloc;
pub mod prelude;
mod sv39;
mod uaccess;
mod vm;

use crate::{debug, log};

// All symbols are from the linker script and are virtual
// addresses.
unsafe extern "C" {
    fn kernel_start();
    fn text_start();
    fn text_end();
    fn boot_pgt();
    fn kernel_stack_start();
    fn kernel_stack_end();
    fn rodata_start();
    fn rodata_end();
    fn data_start();
    fn data_end();
    fn bss_start();
    fn bss_end();
    fn kernel_end();
}

/// The (base_pa, size_bytes) pairs of the QEMU virt machine
/// MMIO scheme. For more details, see [here].
///
/// [here]: https://github.com/qemu/qemu/blob/master/hw/riscv/virt.c#L82
const QEMU_VIRT_MMIO: &[(usize, usize)] = &[
    (0x0010_0000, 0x0000_1000), // VIRT_TEST
    (0x0010_1000, 0x0000_1000), // VIRT_RTC
    (0x0200_0000, 0x0001_0000), // VIRT_CLINT
    (0x0c00_0000, 0x0060_0000), // VIRT_PLIC
    (0x1000_0000, 0x0000_0100), // VIRT_UART0
    (0x1001_0000, 0x0000_1000), // VIRT_VIRTIO
];

const MEM_START_PA: usize = 0x8000_0000;
const MEM_SIZE_BYTES: usize = 128 << 20; // 128 MiB

const PAGE_SIZE_ORDER: usize = 12;
const PAGE_SIZE_BYTES: usize = 1 << PAGE_SIZE_ORDER; // 4 KiB
const LARGE_PAGE_SIZE_ORDER: usize = 21;
const LARGE_PAGE_SIZE_BYTES: usize = 1 << LARGE_PAGE_SIZE_ORDER; // 2 MiB

const KERNEL_VA_OFFSET: usize = 0xffff_ffc0_0000_0000;
const KERNEL_HEAP_SIZE_BYTES: usize = 4 << 20; // 4 MiB

// Subtract one page because 0x40_0000_0000 is not a
// valid virtual address in Sv39.
const USER_SPACE_END: usize = 0x40_0000_0000 - PAGE_SIZE_BYTES;
const USER_STACK_MAX_SIZE_BYTES: usize = 8 << 20; // 8 MiB

pub(crate) fn init() {
    clear_bss();
    heap_alloc::init();
    vm::init_kernel_satp().expect("Failed to activate kernel satp");
    vm::activate_kernel_space();
}

fn clear_bss() {
    (bss_start as usize..bss_end as usize).for_each(|a| {
        unsafe { (a as *mut u8).write_volatile(0) };
    });
}

/// Obtains a pointer for reading the physical address
/// `pa` under the kernel's satp.
fn get_pa_mut_ptr(pa: usize) -> *mut u8 {
    get_va_from_pa(pa) as *mut u8
}

/// Returns the virtual address of the given `pa` under
/// the kernel's satp.
fn get_va_from_pa(pa: usize) -> usize {
    pa.checked_add(KERNEL_VA_OFFSET).expect("address overflow")
}

/// Returns the physical address of the given `va` under
/// the kernel's satp.
fn get_pa_from_va(va: usize) -> usize {
    va.checked_sub(KERNEL_VA_OFFSET).expect("address underflow")
}

/// Physical Page Number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PPN(usize);

impl PPN {
    /// Sv39 limits the physical address to 56 bits.
    const MAX_PA: usize = (1 << 56) - 1;

    fn from_pa(pa: usize) -> Self {
        assert!(Self::is_valid_pa(pa), "invalid physical address");
        PPN(pa >> PAGE_SIZE_ORDER)
    }

    fn is_valid_pa(pa: usize) -> bool {
        pa < Self::MAX_PA
    }

    fn get_pa(&self) -> usize {
        self.0 << PAGE_SIZE_ORDER
    }

    /// Returns the inner value of this [PPN].
    fn get_raw(&self) -> usize {
        self.0
    }
}

/// Virtual Page Number
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct VPN(usize);

impl VPN {
    pub(crate) fn from_va(va: usize) -> VPN {
        assert!(Self::is_valid_va(va), "invalid virtual address");
        VPN(va >> PAGE_SIZE_ORDER)
    }

    fn is_valid_va(va: usize) -> bool {
        // va must have bits 63-39 all equal to bit 38 (0-indexed)
        // according to the Sv39 scheme.
        let mask = 0xffff_ffc0_0000_0000;
        va & mask == 0 || va & mask == mask
    }

    fn get_va(&self) -> usize {
        self.0 << PAGE_SIZE_ORDER
    }
}

pub(crate) fn log_kernel_layout() {
    debug!(
        " kernel [{:#x}, {:#x}) size={}",
        kernel_start as usize,
        kernel_end as usize,
        kernel_end as usize - kernel_start as usize
    );
    debug!(
        ".text [{:#x}, {:#x}) size={}",
        text_start as usize,
        text_end as usize,
        text_end as usize - text_start as usize
    );
    debug!(
        ".boot_pgt [{:#x}, {:#x}) size={}",
        boot_pgt as usize,
        boot_pgt as usize + PAGE_SIZE_BYTES,
        PAGE_SIZE_BYTES,
    );
    debug!(
        ".kernel_stack [{:#x}, {:#x}) size={}",
        kernel_stack_start as usize,
        kernel_stack_end as usize,
        kernel_stack_end as usize - kernel_stack_start as usize
    );
    debug!(
        ".rodata [{:#x}, {:#x}) size={}",
        rodata_start as usize,
        rodata_end as usize,
        rodata_end as usize - rodata_start as usize
    );
    debug!(
        ".data [{:#x}, {:#x}) size={}",
        data_start as usize,
        data_end as usize,
        data_end as usize - data_start as usize
    );
    debug!(
        ".bss [{:#x}, {:#x}) size={}",
        bss_start as usize,
        bss_end as usize,
        bss_end as usize - bss_start as usize
    );
}
