//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    #[allow(unused)]
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        if let Some(task) = self.ready_queue.front() {
            let mut str: isize = 0;
            let mut str_sti = isize::MAX;
            for (i, task) in self.ready_queue.iter().enumerate() {
                if task.inner_exclusive_access().stride < str_sti {
                    str_sti = task.inner_exclusive_access().stride;
                    str = i as isize;
                }
            }
            for i in 0..1000 {
                if i == str {
                    break;
                }
                let taskk = self.ready_queue.pop_front().unwrap();
                self.ready_queue.push_back(taskk);
            }
            let mut taskk = self.ready_queue.pop_front().unwrap();
            let pro = taskk.inner_exclusive_access().priority;
            taskk.inner_exclusive_access().stride += 1000 / pro;
            Some(taskk)
        } else {
            None
        }
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
