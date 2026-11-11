//! Process management syscalls
use crate::{
    config::PAGE_SIZE,
    mm::{
        translated_byte_buffer, MapPermission, PTEFlags, PageTable, VirtAddr},
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next, get_syscall_trace, suspend_current_and_run_next, mmap, munmap
    },
    timer::get_time_us
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let len = core::mem::size_of::<TimeVal>();
    let ts = translated_byte_buffer(
        current_user_token(), 
        ts as *const u8, 
        len
    );
    let us = get_time_us();
    let tv = TimeVal {
        usec : us % 1_000_000,
        sec  : us / 1_000_000,
    };
    let src = unsafe {
        core::slice::from_raw_parts(&tv as *const TimeVal as *const u8, len)
    };
    if ts.iter().map(|x| x.len()).sum::<usize>() != len {
        return -1;
    }
    let mut written = 0usize;
    for slice in ts {
        let n = slice.len().min(len - written);
        slice[..n].copy_from_slice(&src[written..written + n]);
        written += n;
        if written == len {
            break;
        }
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 => {
            let table = PageTable::from_token(current_user_token());
            let vpn = VirtAddr::from(id).floor();
            match table.translate(vpn){
                Some(pte) => {
                    if !pte.is_valid() || !pte.readable() || 
                        !pte.flags().contains(PTEFlags::U) {
                        -1
                    } else {
                        let ppn = pte.ppn();
                        ppn.get_bytes_array()[id & (PAGE_SIZE - 1)] as isize
                    }
                },
                _ => { -1 }
            }
        },
        1 => {
            let table = PageTable::from_token(current_user_token());
            let vpn = VirtAddr::from(id).floor();
            match table.translate(vpn) {
                Some(pte) => {
                    if !pte.is_valid() || !pte.writable() ||
                        !pte.readable() || !pte.flags().contains(PTEFlags::U) {
                        -1
                    } else {
                        let ppn = pte.ppn();
                        ppn.get_bytes_array()[id & (PAGE_SIZE - 1)] = data as u8;
                        0
                    }
                },
                _ => { -1 }
            }
        },
        2 => {
            get_syscall_trace(id) as isize
        },
        _ => { -1 },
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");
    let start = start.into();
    if port == 0 || port & !0x7 != 0 {
        return -1;
    }
    let perm = 
        match MapPermission::from_bits((port << 1) as u8 | MapPermission::U.bits()) {
            Some(p) => { p },
            None => { return -1; },
        };
    if mmap(start, len, perm) {
        0
    } else { -1 }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    let start = start.into();
    if munmap(start, len) {
        0
    } else { -1 }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
