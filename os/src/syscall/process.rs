//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    mm::translated_byte_buffer,
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next, get_first_schedule_time,
        get_syscall_times, mmap, suspend_current_and_run_next, unmmap, TaskStatus,
    },
    timer::{get_time_ms, get_time_us},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
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
use core::mem::size_of;
#[allow(unused)]
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        let mut vvv = translated_byte_buffer(
            current_user_token(),
            (_ts as *const u8),
            size_of::<TimeVal>(),
        );
        let mut buffer = [0u8; size_of::<TimeVal>()];

        let rptr = buffer.as_mut_ptr() as usize as *mut TimeVal;
        *rptr = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };

        for vv in vvv.iter_mut() {
            for i in 0..vv.len() {
                vv[i] = buffer[i];
            }
        }
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
#[allow(unused)]
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    unsafe {
        let mut vvv = translated_byte_buffer(
            current_user_token(),
            (_ti as *const u8),
            size_of::<TaskInfo>(),
        );
        let mut buffer = [0u8; size_of::<TaskInfo>()];

        let rptr = buffer.as_mut_ptr() as usize as *mut TaskInfo;
        (*rptr).status = TaskStatus::Running;
        for i in 0..MAX_SYSCALL_NUM {
            (*rptr).syscall_times[i] = get_syscall_times(i);
        }
        (*rptr).time = get_time_ms() - get_first_schedule_time();

        for vv in vvv.iter_mut() {
            for i in 0..vv.len() {
                vv[i] = buffer[i];
            }
        }
    }
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    if _port & !0x7 != 0 || _port & 0x7 == 0 || _start % 4096 != 0 {
        return -1;
    } else {
        let mut ll = _len;
        if ll % 4096 != 0 {
            ll = (ll / 4096 + 1) * 4096;
        }
        mmap(_start, ll, _port)
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    if _start % 4096 != 0 {
        return -1;
    }
    let mut ll = _len;
        if ll % 4096 != 0 {
            ll = (ll / 4096 + 1) * 4096;
        }
    unmmap(_start, ll)
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
