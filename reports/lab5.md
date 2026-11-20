1. 在每个有关锁/信号量的操作中维护数据结构 avail alloc need, 在 PCB 中定义这样的三个 field, 维护数据结构在每次请求锁/信号量的操作中用这些数据结构 check 是否死锁，如果死锁直接返回 -0xDEAD 即可。对于信号量而言，我选择了在每次 down 的时候重新维护 need 数组，由于线程被 up 唤醒后再被调度才会清除他的 need, 如果没调度之前发生了 down 的请求，这时候会让 need 虚高（实际未被该线程需要或已经释放），所以每次重新维护 need 即可。
    时间约 2-3h

2. 
    - 分为每个线程自己的资源和整个进程的资源。
        - 每个线程：
            - 内核栈：由 drop 方法回收
            - 用户资源：由 drop 方法回收
            - TCB：由最后的 Arc 引用的 drop 方法回收
            - trap context 物理页：由 memory_set 回收
        - 整个进程：
            - 代码/数据页（页表等），文件描述符表，pid，子进程（转交给 init），mutex_list, semaphore_list / condvar_list 随着 PCB drop，PCB 本身设置 is_zombie，等 wait 的父进程释放
    - 其他线程的 TCB 被引用的位置：
        - PCB 的 tasks 中，需要回收，调用 clear 方法
        - PROCESSOR 的 current, 需要回收，直接 take 拿走所有权即可
        - TASK_MANAGER 的 ready_queue，需要回收, remove_task
        - TIMERS，需要回收，remove_timer
        - 各种信号量/锁/条件变量中的 wait_queue，不需要回收，等待对应的list释放即可
        - TASK_MANAGER 的 stop_task，需要回收，主进程退出的时候不能立即 drop, 需要挂到 stop_task上，等 __switch 到别的进程后再回收
        - TCBInner 中持有的，不需要回收，均为弱引用/栈上的数据

    - 
        - mutex1 把 locked 设置为 false, 再唤醒一个等待者，唤醒者需要回到 loop 中重新竞争
        - mutex2 把所有等待者保持为 locked = true，把锁交给被唤醒者，返回就认为已经拿到锁
        - mutex1:
            - 可能会导致某线程饥饿
            - 可能会导致同一把锁的唤醒顺序不同
            - 对伪唤醒安全
        - mutex2:
            - 伪唤醒会破坏互斥，特别是 riscv 架构
            - 必须保证被唤醒的任务一定能被调度运行
            - 公平，切换较少
    
