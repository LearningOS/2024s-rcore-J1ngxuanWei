//! File and filesystem-related syscalls

use crate::fs::{
    new_fromname, open_file, rmv_fromna, rmv_fromno, OpenFlags, Stat, OSINODE_MANAGER,
};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        if fd == 0 || fd == 1 {
            return file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize;
        }
        let namm = file.get_name();
        //println!("sys_write: {}", namm);
        if let Some(sta) = OSINODE_MANAGER.exclusive_access().find_inode(&namm) {
            sta.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
        } else {
            //println!("error sys_write");
            -1
        }
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        if fd == 0 || fd == 1 {
            return file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize;
        }
        let namm = file.get_name();
        if let Some(sta) = OSINODE_MANAGER.exclusive_access().find_inode(&namm) {
            sta.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
        } else {
            //println!("error sys_read");
            -1
        }
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if fd == 0 || fd == 1 {
        inner.fd_table[fd].take();
        return 0;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        let namm = file.get_name();
        let mut ino: u64 = 0;
        let mut nlink: u32 = 0;
        let mut fl = false;
        if let Some(sta) = OSINODE_MANAGER.exclusive_access().find_inode(&namm) {
            let stat = sta.get_stat();
            ino = stat.ino;
            nlink = stat.nlink - 1;
            fl = true;
        }
        if fl {
            rmv_fromno(ino, nlink, namm);
            inner.fd_table[fd].take();
        }
    } else {
        return -1;
    }
    0
}

use core::mem::size_of;
/// YOUR JOB: Implement fstat.
#[allow(unused)]
pub fn sys_fstat(fd: usize, _st: *mut Stat) -> isize {
    trace!(
        "kernel:pid[{}] sys_fstat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        //println!("11");
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        unsafe {
            let mut vvv =
                translated_byte_buffer(current_user_token(), (_st as *const u8), size_of::<Stat>());
            let namm = file.get_name();
            //println!("sys_fstat: {}", namm);
            if let Some(sta) = OSINODE_MANAGER.exclusive_access().find_inode(&namm) {
                let stat = sta.get_stat();
                //println!("sys_fstat: {}", stat.nlink);
                let mut buffer = [0u8; size_of::<Stat>()];
                let rptr = buffer.as_mut_ptr() as usize as *mut Stat;
                *rptr = Stat {
                    dev: stat.dev,
                    ino: stat.ino,
                    mode: stat.mode,
                    nlink: stat.nlink,
                    pad: [0; 7],
                };
                for vv in vvv.iter_mut() {
                    for i in 0..vv.len() {
                        vv[i] = buffer[i];
                    }
                }
            }
        }
        0
    } else {
        //println!("33");
        -1
    }
}

/// YOUR JOB: Implement linkat.
#[allow(unused)]
pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_linkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let task = current_task().unwrap();
    let token = current_user_token();
    let oldna = translated_str(token, _old_name);
    let newna = translated_str(token, _new_name);
    if let Some(a) = new_fromname(&oldna, &newna) {
        0
    } else {
        -1
    }
}

/// YOUR JOB: Implement unlinkat.
#[allow(unused)]
pub fn sys_unlinkat(_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_unlinkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let task = current_task().unwrap();
    let token = current_user_token();
    let na = translated_str(token, _name);
    //println!("sys_unlinkat: {}", na);
    rmv_fromna(na)
}
