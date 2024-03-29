use crate::task::global::task_manager;
use crate::task::TaskID;
use core::arch::asm;
use core::ops::{Deref, DerefMut};
use heapless::mpmc::Q32;

/// A Mutex wrapper to protect from deadlocks caused by the design of TaskManager.
///
/// When a timer interrupt occurs, [crate::task::TaskManager]
/// 1. finds the next task from a queue that is not empty in descending order of priority level
/// 2. makes the current task sleep
/// 3. wakes the next task up
///
/// Due to this design,
/// if a low priority task holds Mutex's lock and a higher priority task needs to wait until the lock is released,
/// TaskManager never wakes the lower-priority task up and this leads to deadlock.
pub struct Mutex<T> {
    inner: spin::Mutex<T>,
    // use a fixed size queue to use the Mutex wrapper to MemoryManager
    queue: Q32<TaskID>,
}

pub struct MutexGuard<'a, T> {
    inner: spin::MutexGuard<'a, T>,
    queue: &'a Q32<TaskID>,
}

impl<T> Mutex<T> {
    pub const fn new(value: T) -> Self {
        Self {
            inner: spin::Mutex::new(value),
            queue: Q32::new(),
        }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        self.inner.try_lock().map(|inner| MutexGuard {
            inner,
            queue: &self.queue,
        })
    }

    pub fn lock(&self) -> MutexGuard<T> {
        let inner_guard = loop {
            if let Some(guard) = self.inner.try_lock() {
                break guard;
            } else {
                unsafe { asm!("cli") };
                let task_id = task_manager().current_task().id();
                self.queue
                    .enqueue(task_id)
                    .expect("failed to enqueue a task id");

                task_manager()
                    .sleep(task_id)
                    .expect("failed to sleep a task");
                unsafe { asm!("sti") };
            }
        };

        MutexGuard {
            inner: inner_guard,
            queue: &self.queue,
        }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        while let Some(id) = self.queue.dequeue() {
            unsafe { asm!("cli") };
            let _ = task_manager().wake_up(id);
            unsafe { asm!("sti") };
        }
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.inner
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}
