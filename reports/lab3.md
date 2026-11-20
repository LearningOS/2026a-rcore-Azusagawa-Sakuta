1. 
    - sys_get_time, sys_mmap, sys_munmap:
        - 这些系统调用和 ch4 几乎一致，因为并没有改变地址空间的结构
    - sys_spawn
        - 类似于把 create, fork 和 exec 的代码拼合，其中 create 中提到了如何通过 name 创建一个进程，fork 中提到了如何在当前进程下创建一个子进程，exec 中提到了如何把子进程加入任务轮训。
    - stride 调度算法
        - 只需要在 TCB 中维护 stride 和 priority 字段就可以了，剩下的按照说明更新这些字段就可以了。我选择在 fork 后继承父进程的 priority, 但不集成 stride, 但后来写到后面几章后觉得会让其他线程饥饿，在后面的时候选择继承 stride, 这样可以尽量均衡地调度线程

2. 问答题
    - 不是，这时候 p2 的 stride 由于溢出已经变成了 4, 但是 p1 的 stride 还在 255, 由于调度算法选择的是 stride 较小的进程，导致在 p1 永远无法被调度，始终处于饥饿状态。
    - 每次更新 stride 的时候增加的步数最大是 BigStride / 2, 可知初始状态下差距小于最大步数的永远不会超过这个差距，如果初始状态差距大于最大步数，则大的进程等小的执行到与其差距小于最大步数后，又会回到第一种情况，故差距不可能一直超过最大步数
    - 代码如下
        ```rust
        use core::cmp::Ordering;

        struct Stride(u64);

        impl PartialOrd for Stride {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                let diff = self.0.wrapping_sub(other.0) as i64;
                if diff > 0 {
                    if diff > BIG_STRIDE / 2 {
                        Some(Ordering::Less)
                    } else {
                        Some(Ordering::Greater)
                    }
                } else {
                    if diff < -(BIG_STRIDE / 2) {
                        Some(Ordering::Greater)
                    } else {
                        Some(Ordering::Less)
                    }
                }
            }
        }

        impl PartialEq for Stride {
            fn eq(&self, other: &Self) -> bool {
                false
            }
        }
        ```

3. 荣誉准则

    1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

    2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

            《你参考的资料说明》

    3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

    4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

