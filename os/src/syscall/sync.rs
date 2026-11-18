use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task, ProcessControlBlockInner};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;

/// check if deadlock will happen
fn check(avail: &Vec<usize>, alloc: &Vec<Vec<usize>>, need: &Vec<Vec<usize>>) -> bool {
    let n = alloc.len();
    let m = avail.len();
    let mut work = avail.clone();
    let mut finish = vec![false; n];
    loop {
        let mut ok = false;
        for i in 0..n {
            if finish[i] { continue; }
            if (0..m).all(|j| need[i][j] <= work[j]) {
                for j in 0..m {
                    work[j] += alloc[i][j];
                }
                finish[i] = true;
                ok = true;
            }
        }
        if ok == false {
            break;
        }
    }
    finish.iter().all(|x| *x)
}
/// ensure dims of alloc and need
fn ensure_dims(
    inner: &mut ProcessControlBlockInner,
    tid: usize,
    m: usize,
) {
    let alloc = &mut inner.alloc as *mut Vec<Vec<usize>>;
    let need  = &mut inner.need  as *mut Vec<Vec<usize>>;
    unsafe {
        while (*alloc).len() <= tid { (*alloc).push(vec![0; m]); }
        while  (*need).len() <= tid {  (*need).push(vec![0; m]); }
        for row in (*alloc).iter_mut() { 
            while row.len() < m { row.push(0); }
        }
        for row in  (*need).iter_mut() {
            while row.len() < m { row.push(0); }
        }
    }
}
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid,
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    let m = process_inner.mutex_list.len();
    ensure_dims(
        &mut process_inner, tid, m
    );
    while process_inner.avail.len() < m { process_inner.avail.push(0); }
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.avail[id] = 1;
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.avail.push(1);
        for row in process_inner.alloc.iter_mut() {
            row.push(0);
        }
        for row in process_inner.need.iter_mut() {
            row.push(0);
        }
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid,
    );

    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let m = process_inner.mutex_list.len();
    ensure_dims(&mut process_inner, tid, m);
    let mut wait = false;
    if process_inner.enable_detection {
        if process_inner.avail[mutex_id] > 0 {
            // Try to alloc
            process_inner.avail[mutex_id] -= 1;
            process_inner.alloc[tid][mutex_id] += 1;
            if !check(
                &process_inner.avail,
                &process_inner.alloc,
                &process_inner.need
            ) {
                process_inner.avail[mutex_id] += 1;
                process_inner.alloc[tid][mutex_id] -= 1;
                return -0xDEAD;
            }
        } else {
            // block
            process_inner.need[tid][mutex_id] += 1;
            if !check(
                &process_inner.avail,
                &process_inner.alloc,
                &process_inner.need,
            ) {
                process_inner.need[tid][mutex_id] -= 1;
                return -0xDEAD;
            }
            wait = true;
        }
    }
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.lock();
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.enable_detection && wait {
        process_inner.need[tid][mutex_id] -= 1;
        process_inner.alloc[tid][mutex_id] += 1;
    }
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid,
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.enable_detection {
        let m = process_inner.mutex_list.len();
        ensure_dims(&mut process_inner, tid, m);
        process_inner.alloc[tid][mutex_id] -= 1;
        process_inner.avail[mutex_id] += 1;
    }
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid,
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let m = process_inner.semaphore_list.len();
    ensure_dims(
        &mut process_inner, tid, m
    );
    while process_inner.avail.len() < m { process_inner.avail.push(0); }
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.avail[id] = res_count;
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner.avail.push(res_count);
        for row in process_inner.alloc.iter_mut() {
            row.push(0);
        }
        for row in process_inner.need.iter_mut() {
            row.push(0);
        }
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid,
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    let cnt = sem.inner.exclusive_access().count;
    drop(process_inner);
    sem.up();
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.enable_detection {
        let m = process_inner.semaphore_list.len();
        ensure_dims(&mut process_inner, tid, m);
        process_inner.alloc[tid][sem_id] -= 1;
        process_inner.avail[sem_id] = if cnt > 0 { cnt as usize } else { 0 };
    }
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid,
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    let cnt = sem.inner.exclusive_access().count;
    process_inner.avail[sem_id] = if cnt > 0 { cnt as usize } else { 0 };
    let m = process_inner.semaphore_list.len();
    ensure_dims(&mut process_inner, tid, m);
    let mut wait = false;
    if process_inner.enable_detection {
        // Derive `need` from the wait queues: a task that is still waiting on a
        // semaphore has an unsatisfied request; once it is woken it is removed
        // from the queue, so its need disappears immediately (no stale state).
        let n = process_inner.alloc.len();
        let mm = process_inner.avail.len();
        let mut need = vec![vec![0usize; mm]; n];
        for (j, sem_opt) in process_inner.semaphore_list.iter().enumerate() {
            if j >= mm {
                break;
            }
            if let Some(s) = sem_opt {
                let inner = s.inner.exclusive_access();
                for task in inner.wait_queue.iter() {
                    let t = task
                        .inner_exclusive_access()
                        .res
                        .as_ref()
                        .unwrap()
                        .tid;
                    if t < n {
                        need[t][j] = 1;
                    }
                }
            }
        }
        if process_inner.avail[sem_id] > 0 {
            // Try to allocate
            process_inner.avail[sem_id] -= 1;
            process_inner.alloc[tid][sem_id] += 1;
            if !check(&process_inner.avail, &process_inner.alloc, &need) {
                process_inner.avail[sem_id] += 1;
                process_inner.alloc[tid][sem_id] -= 1;
                return -0xDEAD;
            }
        } else {
            // Will block: record this request
            need[tid][sem_id] += 1;
            if !check(&process_inner.avail, &process_inner.alloc, &need) {
                return -0xDEAD;
            }
            wait = true;
        }
    }
    drop(process_inner);
    sem.down();
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.enable_detection && wait {
        process_inner.alloc[tid][sem_id] += 1;
    }
    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect");
    match enabled {
        0 | 1 => {
            let process = current_process();
            let mut process_inner = process.inner_exclusive_access();
            process_inner.enable_detection = enabled == 1;
            0
        },
        _ => { -1 },
    }
}
