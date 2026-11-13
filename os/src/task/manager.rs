//!Implementation of [`TaskManager`]
use core::cmp::Ordering;

use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::BinaryHeap;
use alloc::sync::Arc;
use lazy_static::*;
use crate::config::BIG_STRIDE;

/// new type wrapper for [`TaskManager`]
pub struct TCBs(pub Arc<TaskControlBlock>);

///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: BinaryHeap<TCBs>,
}

impl Ord for TCBs {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.inner_exclusive_access().stride.cmp(
            &self.0.inner_exclusive_access().stride
        )
            .then_with(|| other.0.getpid().cmp(&self.0.getpid()))
    }
}

impl PartialEq for TCBs {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for TCBs {}

impl PartialOrd for TCBs {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: BinaryHeap::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push(TCBs(task));
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let task = self.ready_queue.pop()?.0;
        let task_clone = task.clone();
        let mut inner = 
            task_clone.inner_exclusive_access();
        inner.stride += BIG_STRIDE / inner.priority;
        Some(task)
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
