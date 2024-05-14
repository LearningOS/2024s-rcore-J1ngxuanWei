# lab5

## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

无

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

## 实现的功能

利用银行家算法实现死锁检测。

所用时间：大约4-5小时

## 在我们的多线程实现中，当主线程 (即 0 号线程) 退出时，视为整个进程退出， 此时需要结束该进程管理的所有线程并回收其资源。 - 需要回收的资源有哪些？ - 其他线程的 TaskControlBlock 可能在哪些位置被引用，分别是否需要回收，为什么？

对于已经结束的线程，需要回收其控制块的内存，对于没有结束的，我们需要回收其还在使用的所有内存。

然后需要回收进程本身的控制块或是其他与进程相关的进程层面的内存，因为本质上是进程的退出。

其他的线程的控制块可能在其他正在运行的线程或是其顶层的进程所使用，因为可能需要访问一部分数据，往往所有的控制块是在进程结束回收。

## 对比以下两种 Mutex.unlock 的实现，二者有什么区别？这些区别可能会导致什么问题？

```
impl Mutex for Mutex1 {
     fn unlock(&self) {
         let mut mutex_inner = self.inner.exclusive_access();
         assert!(mutex_inner.locked);
         mutex_inner.locked = false;
         if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
             add_task(waking_task);
         }
    }
}

impl Mutex for Mutex2 {
    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            add_task(waking_task);
        } else {
          mutex_inner.locked = false;
        }
    }
}
```
区别在于等待队列的前面所弹出的是None时的处理方法不同，一个不作操作，一个将锁打开。

首先，第二种是对的，第一种会在等待队列为空时，也就是弹出None时，不进行锁的打开，此时会使得这个锁不能再被获取，导致锁只能使用一次。
