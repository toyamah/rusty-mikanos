use crate::task::global::task_manager;
use crate::task::TaskID;
use core::arch::asm;
use core::ops::{Deref, DerefMut};
use heapless::mpmc::Q32;

/// A Mutex wrapper to protect from deadlocks caused by the design of TaskManager.
pub struct Mutex<T> {
    inner: spin::Mutex<T>,
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
