extern crate alloc;

use alloc::vec::Vec;
use buddy_system_allocator::LockedHeap;

use crate::mm::{bss_end, bss_start};

static KERNEL_HEAP_SIZE: usize = 4 * 1024 * 1024;
static mut KERNEL_HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<23> = LockedHeap::empty();

pub(super) fn init() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init((&raw const KERNEL_HEAP).addr(), KERNEL_HEAP_SIZE);
    }
}

#[allow(dead_code)]
pub(crate) fn test_heap() {
    let bss_range = (bss_start as usize)..(bss_end as usize);
    let mut v1 = Vec::<usize>::new();
    let mut v2 = alloc::vec![2077];

    for i in 0..100 {
        v1.push(i);
    }

    assert!(bss_range.contains(&(v1.as_ptr().addr())));
    assert_eq!(v1.len(), 100);
    for i in 0..100 {
        assert_eq!(v1[i], i);
    }

    assert!(bss_range.contains(&(v2.as_ptr().addr())));
    assert_eq!(v2.len(), 1);
    assert_eq!(v2.pop(), Some(2077));
    assert_eq!(v2.pop(), None);
}
