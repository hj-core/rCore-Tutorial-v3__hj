use crate::mm::prelude::{
    MapType, PAGE_SIZE_BYTES, PERMISSION_R, PERMISSION_U, PERMISSION_W, PERMISSION_X, VPN,
    check_u_va_range,
};
use crate::task::prelude::{get_current_task_id, update_tcb};

const ALL_PROT_FLAGS: usize = PROT_EXEC | PROT_READ | PROT_WRITE;
const PROT_EXEC: usize = 1;
const PROT_READ: usize = 2;
const PROT_WRITE: usize = 4;

pub(super) fn mmap(addr: usize, len: usize, prot: usize) -> isize {
    if !check_u_va_range(addr, len) {
        return -1;
    }

    if prot & !ALL_PROT_FLAGS != 0 {
        return -1;
    }

    let task_id = get_current_task_id();
    let mut result = Ok(());

    update_tcb(task_id, |tcb| {
        let start_vpn = VPN::from_va(addr);
        let end_vpn = VPN::from_va(addr + len + PAGE_SIZE_BYTES - 1);
        let map_type = MapType::Anonymous;
        let permissions = to_permissions(prot);

        result = tcb
            .get_vm_space_mut()
            .add_new_area(start_vpn, end_vpn, map_type, permissions);
    });

    if result.is_ok() {
        return 0;
    } else {
        return -1;
    }
}

fn to_permissions(prot: usize) -> usize {
    let mut result = PERMISSION_U;
    if prot & PROT_READ != 0 {
        result |= PERMISSION_R;
    }
    if prot & PROT_WRITE != 0 {
        result |= PERMISSION_W;
    }
    if prot & PROT_EXEC != 0 {
        result |= PERMISSION_X;
    }
    result
}
