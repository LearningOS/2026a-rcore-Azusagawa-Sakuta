- 简单总结你实现的功能:
    - 实现了一个 syscall 的 tracer ， 每次在进行 syscall 的时候会记录对应的 syscall_id 的次数，用 TaskControlBlock 中新加入的 syscall_times 数组记录，下标为 syscall_id ，数组内容为对应 id 的次数。在 task 中写了两个函数完成对应 times 的增加和获取。
    - 遇到了一个问题，在运行 BASE=0 和 BASE=1 时均可通过，但 BASE=2 时 sleep.rs 发生了 assert failed panic，我尝试了各种方法无法通过，但是经过 ds 检查后，发现是 TaskControlBlock 中加入的 syscall_times 数组太大，将 .data 中的除法魔数清零了，将数组改小后正常运行了。

- 问答题：
    1. 用 make run BASE=1 CHAPTER=2 运行 ch2_bad_* 三个测例，三个程序都未打印各自的 FAIL: T.T，而是在 U 态触发异常后被内核接管，向地址 0 写数据触发存储访问异常，内核打印 PageFault， 在 U 态执行 sret，在 U 态读取 S 态 CSR sstatus，都触发非法指令异常（scause=2），内核打印 IllegalInstruction in application, kernel killed it. 说明 U 态既不能执行 S 态特权指令，也不能访问 S 态寄存器，特权级隔离生效。 使用的 SBI 为课程提供的 RustSBI 0.4.1
    2. 
        1. sp 代表 kernel 的 stack 上当前任务的 TrapContext 起始地址.
            - Senario 1: 任务第一次运行时，goto_restore 把 ra 设成 \_\_restore，\_\_switch ret 跳到 \_\_restore
            - Senario 2: 从 trap 返回，\_\_alltraps 里 call trap_handler 返回后，回到\_\_restore
        2. 特殊处理了 sstatus, sepc, sscratch
            - sstatus: 把 trap 发生那一刻的 sstatus 恢复回去，决定 sret 之后的行为
            - sepc: 让执行流返回到中断发生或发生后的位置继续执行
            - sscratch: 恢复用户栈指针
        3. x2 是 sp，如果恢复 x2 后面那些 ld 就不能取到正确位置, x4 是 tp, 用户程序不用 tp, 内核自己也不用
        4. sp 指向用户栈，sscratch 指向内核栈
        5. sret, PC <- sepc、特权级 <- sstatus.SPP、SIE <- sstatus.SPIE，因为 SPP 是 U，sret 之后就从执行 S 态代码变成从 sepc 处执行 U 态代码
        6. sp 指向内核栈，sscratch 指向用户栈
        7. ecall, 硬件在响应异常时完成的

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
