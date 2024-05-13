//! `Arc<Inode>` -> `OSInodeInner`: In order to open files concurrently
//! we need to wrap `Inode` into `Arc`,but `Mutex` in `Inode` prevents
//! file systems from being accessed simultaneously
//!
//! `UPSafeCell<OSInodeInner>` -> `OSInode`: for static `ROOT_INODE`,we
//! need to wrap `OSInodeInner` into `UPSafeCell`
use super::{File, Stat, StatMode};
use crate::drivers::BLOCK_DEVICE;
use crate::mm::UserBuffer;
use crate::sync::UPSafeCell;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::*;
use easy_fs::{DirEntry, EasyFileSystem, Inode};
use lazy_static::*;

/// inode in memory
/// A wrapper around a filesystem inode
/// to implement File trait atop
pub struct OSInode {
    readable: bool,
    writable: bool,
    inner: UPSafeCell<OSInodeInner>,
    name: String,
    stat: Stat,
}
/// The OS inode inner in 'UPSafeCell'
pub struct OSInodeInner {
    offset: usize,
    inode: Arc<Inode>,
}

pub struct OSInodeManager {
    inodes: Vec<OSInode>,
}

impl OSInodeManager {
    pub fn new() -> Self {
        Self { inodes: Vec::new() }
    }
    pub fn add_inode(&mut self, inode: OSInode) {
        self.inodes.push(inode);
    }
    pub fn find_inode(&self, name: &str) -> Option<&OSInode> {
        for inode in self.inodes.iter() {
            if inode.name == name {
                return Some(inode);
            }
        }
        None
    }
    #[allow(unused)]
    pub fn fresh(&mut self) {
        let mut vv: Vec<(u64, u64)> = Vec::new();
        for (i, inode) in self.inodes.iter().enumerate() {
            //println!("name: {}, ino: {}, nlink: {}", inode.name, inode.get_ino(), inode.stat.nlink);
            let mut stat = inode.stat;
            let ino = inode.get_ino();
            let mut ll: u64 = 0;
            for j in 0..self.inodes.len() {
                if self.inodes[j].get_ino() == ino {
                    ll += 1;
                }
            }
            vv.push((ino, ll));
        }
        vv.sort_by(|a, b| a.1.cmp(&b.1));
        let mut vfv: Vec<(u64, u64)> = Vec::new();
        vfv.push((vv[0].0, vv[0].1));
        for i in vv.iter() {
            let l = vfv.len() - 1;
            if i.0 == vfv[l].0 {
                vfv[l].1 = i.1;
            } else {
                vfv.push((i.0, i.1));
            }
        }
        for i in vfv.iter() {
            for iin in self.inodes.iter_mut() {
                //println!("name: {}, ino: {}, nlink: {}", iin.name, iin.get_ino(), iin.stat.nlink);
                if iin.get_ino() == i.0 {
                    iin.set_link(i.1 as u32);
                }
            }
        }
    }
    #[allow(unused)]
    pub fn fresh_one(&mut self, ino: u64, lik: u32, name: String) {
        //println!("fresh one ino: {}, lik: {}, name: {}", ino, lik, name);
        let mut ind = 0;
        let mut fg = false;
        for (i, iin) in self.inodes.iter_mut().enumerate() {
            if iin.name == name {
                ind = i;
                fg = true;
            }
            if iin.get_ino() == ino {
                iin.set_link(lik);
            }
        }
        if fg {
            self.inodes.remove(ind);
        }
    }
}

impl OSInode {
    /// create a new inode in memory
    pub fn new(readable: bool, writable: bool, inode: Arc<Inode>, name: &str) -> Self {
        let mut dirent = DirEntry::empty();
        let siz = inode.read_at(0, dirent.as_bytes_mut());
        // 下面应该不用考虑DIR，先这样，不过再说
        if siz == 32 {
            Self {
                readable,
                writable,
                inner: unsafe { UPSafeCell::new(OSInodeInner { offset: 0, inode }) },
                stat: Stat {
                    dev: 0,
                    ino: dirent.inode_id() as u64,
                    mode: StatMode::DIR,
                    nlink: 0,
                    pad: [0; 7],
                },
                name: String::from(name),
            }
        } else {
            let na: Vec<String> = ROOT_INODE.ls();
            let mut nub: u64 = 0;
            for i in na.iter() {
                let ff: &str = i;
                if let Some(id) = ROOT_INODE.find_id(ff) {
                    nub = id as u64;
                    break;
                }
            }
            Self {
                readable,
                writable,
                inner: unsafe { UPSafeCell::new(OSInodeInner { offset: 0, inode }) },
                stat: Stat {
                    dev: 0,
                    ino: nub,
                    mode: StatMode::FILE,
                    nlink: 1,
                    pad: [0; 7],
                },
                name: String::from(name),
            }
        }
    }
    /// read all data from the inode
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.exclusive_access();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }
    ///1
    pub fn get_ino(&self) -> u64 {
        self.stat.ino
    }
    ///2
    pub fn set_link(&mut self, nlink: u32) {
        self.stat.nlink = nlink;
    }
    ///3
    pub fn get_stat(&self) -> &Stat {
        &self.stat
    }
    ///4
    pub fn set_readable(&mut self, readable: bool) {
        self.readable = readable;
    }
    ///5
    pub fn set_writable(&mut self, writable: bool) {
        self.writable = writable;
    }
    ///2
    pub fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_read_size = 0usize;
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }
    ///3
    pub fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_write_size = 0usize;
        for slice in buf.buffers.iter() {
            let write_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }
        total_write_size
    }
}

lazy_static! {
    ///1
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
        Arc::new(EasyFileSystem::root_inode(&efs))
    };
    ///1
    pub static ref OSINODE_MANAGER: UPSafeCell<OSInodeManager> =
        unsafe { UPSafeCell::new(OSInodeManager::new()) };
}

/// List all apps in the root directory
pub fn list_apps() {
    println!("/**** APPS ****");
    for app in ROOT_INODE.ls() {
        println!("{}", app);
    }
    println!("**************/");
}

bitflags! {
    ///  The flags argument to the open() system call is constructed by ORing together zero or more of the following values:
    pub struct OpenFlags: u32 {
        /// readyonly
        const RDONLY = 0;
        /// writeonly
        const WRONLY = 1 << 0;
        /// read and write
        const RDWR = 1 << 1;
        /// create new file
        const CREATE = 1 << 9;
        /// truncate file size to 0
        const TRUNC = 1 << 10;
    }
}

impl OpenFlags {
    /// Do not check validity for simplicity
    /// Return (readable, writable)
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}

/// Open a file
#[allow(unused)]
pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let (readable, writable) = flags.read_write();
    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = ROOT_INODE.find(name) {
            // clear size
            inode.clear();
            let tn = Arc::new(OSInode::new(readable, writable, inode.clone(), name));
            let nn = OSInode::new(readable, writable, inode, name);
            OSINODE_MANAGER.exclusive_access().add_inode(nn);
            Some(tn)
        } else {
            // create file
            ROOT_INODE.create(name).map(|inode| {
                let tn = Arc::new(OSInode::new(readable, writable, inode.clone(), name));
                let nn = OSInode::new(readable, writable, inode, name);
                OSINODE_MANAGER.exclusive_access().add_inode(nn);
                tn
            })
        }
    } else {
        if let Some(r) = ROOT_INODE.find(name).map(|inode| {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear();
            }
            let tn = Arc::new(OSInode::new(readable, writable, inode.clone(), name));
            let nn = OSInode::new(readable, writable, inode, name);
            OSINODE_MANAGER.exclusive_access().add_inode(nn);
            tn
        }) {
            //println!("1111");
            Some(r)
        } else {
            //println!("2222");
            let mut fff = OSINODE_MANAGER.exclusive_access();
            if let Some(node) = fff.find_inode(name) {
                let inner = node.inner.exclusive_access();
                let tn = Arc::new(OSInode::new(readable, writable, inner.inode.clone(), name));
                Some(tn)
            } else {
                None
            }
        }
    }
}

///1
#[allow(unused)]
pub fn new_fromname(name: &str, newname: &str) -> Option<Arc<OSInode>> {
    let mut fff = OSINODE_MANAGER.exclusive_access();
    if let Some(oldnode) = fff.find_inode(name) {
        let nn = OSInode::new(
            oldnode.readable,
            oldnode.writable,
            oldnode.inner.exclusive_access().inode.clone(),
            newname,
        );
        let tn = Arc::new(OSInode::new(
            oldnode.readable,
            oldnode.writable,
            oldnode.inner.exclusive_access().inode.clone(),
            newname,
        ));
        fff.add_inode(nn);
        fff.fresh();
        Some(tn)
    } else {
        return None;
    }
}

///2
#[allow(unused)]
pub fn rmv_fromno(ino: u64, lik: u32, name: String) {
    OSINODE_MANAGER.exclusive_access().fresh_one(ino, lik, name);
}

///3
#[allow(unused)]
pub fn rmv_fromna(name: String) -> isize {
    ROOT_INODE.remove(&name).map(|inode| 1);
    let mut ino: u64 = 0;
    let mut lik: u32 = 0;
    if let Some(ii) = OSINODE_MANAGER.exclusive_access().find_inode(&name) {
        ino = ii.get_ino();
        lik = ii.stat.nlink;
    } else {
        //println!("error: file {} not found", name);
    }
    OSINODE_MANAGER
        .exclusive_access()
        .fresh_one(ino, lik - 1, name);
    0
}

impl File for OSInode {
    fn readable(&self) -> bool {
        self.readable
    }
    fn writable(&self) -> bool {
        self.writable
    }
    fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_read_size = 0usize;
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }
    fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_write_size = 0usize;
        for slice in buf.buffers.iter() {
            let write_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }
        total_write_size
    }
    fn get_stat(&self) -> &Stat {
        &self.stat
    }
    fn get_name(&self) -> String {
        self.name.clone()
    }
}
