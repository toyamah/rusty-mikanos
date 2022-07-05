use crate::interrupt::global::notify_end_of_interrupt;
use crate::message::{Message, MessageType};
use crate::task::global::task_manager;
use crate::task::{TaskContext, TaskID, TaskManager};
use crate::timer::global::timer_manager;
use alloc::collections::BinaryHeap;
use core::arch::asm;
use core::cmp::Ordering;
use core::ptr::read_volatile;

const COUNT_MAX: u32 = 0xffffffff;
pub const TIMER_FREQ: u64 = 100;

const TASK_TIMER_PERIOD: u64 = TIMER_FREQ / 50;
// indicates the value for switching a task
const TASK_TIMER_VALUE: i32 = i32::MAX;

pub mod global {
    use super::{divide_config, initial_count, lvt_timer, measure_time, TimerManager, TIMER_FREQ};
    use crate::acpi;
    use crate::interrupt::InterruptVectorNumber;

    static mut TIMER_MANAGER: Option<TimerManager> = None;
    pub fn timer_manager() -> &'static mut TimerManager {
        unsafe { TIMER_MANAGER.as_mut().unwrap() }
    }

    static mut LAPIC_TIMER_FREQ: Option<u64> = None;

    pub fn initialize_lapic_timer() {
        unsafe {
            TIMER_MANAGER = Some(TimerManager::new());
            divide_config().write_volatile(0b1011); // divide 1:1
            lvt_timer().write_volatile(0b001 << 16); // masked, one-shot
        };

        let elapsed = measure_time(|| acpi::global::wait_milliseconds(100));
        let lapic_timer_freq = (elapsed as u64) * 10;
        unsafe { LAPIC_TIMER_FREQ = Some(lapic_timer_freq) };

        unsafe {
            divide_config().write_volatile(0b1011);
            lvt_timer().write_volatile((0b010 << 16) | InterruptVectorNumber::LAPICTimer as u32);
            initial_count().write_volatile((lapic_timer_freq / TIMER_FREQ) as u32);
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn LAPICTimerOnInterrupt(context_stack: *const TaskContext) {
    let context_ref = unsafe { context_stack.as_ref() }.unwrap();
    let task_timer_timeout = timer_manager().tick(task_manager());
    notify_end_of_interrupt();
    if task_timer_timeout {
        task_manager().switch_task(context_ref);
    }
}

pub fn measure_time<F>(f: F) -> u32
where
    F: FnOnce(),
{
    start_lapic_timer();
    f();
    let time = lapic_timer_elapsed();
    stop_lapic_timer();
    time
}

fn start_lapic_timer() {
    unsafe { initial_count().write_volatile(COUNT_MAX) }
}

fn lapic_timer_elapsed() -> u32 {
    unsafe { COUNT_MAX - *current_count() }
}

fn stop_lapic_timer() {
    unsafe { initial_count().write_volatile(0) }
}

fn lvt_timer() -> *mut u32 {
    0xfee00320 as *mut u32
}

fn initial_count() -> *mut u32 {
    0xfee00380 as *mut u32
}

fn current_count() -> *mut u32 {
    0xfee00390 as *mut u32
}

fn divide_config() -> *mut u32 {
    0xfee003e0 as *mut u32
}

#[derive(Debug, Copy, Clone)]
pub struct Timer {
    timeout: u64,
    value: i32,
    task_id: Option<TaskID>,
}

impl Timer {
    pub fn new(timeout: u64, value: i32, task_id: TaskID) -> Timer {
        Self {
            timeout,
            value,
            task_id: Some(task_id),
        }
    }

    fn sentinel() -> Self {
        Self {
            // the sentinel timer is the longest timeout
            timeout: u64::MAX,
            value: 0,
            task_id: None,
        }
    }
}

impl Eq for Timer {}

impl PartialEq<Self> for Timer {
    fn eq(&self, other: &Self) -> bool {
        other.timeout == self.timeout
    }
}

impl PartialOrd<Self> for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.timeout.partial_cmp(&self.timeout)
    }
}

impl Ord for Timer {
    fn cmp(&self, other: &Self) -> Ordering {
        other.timeout.cmp(&self.timeout)
    }
}

#[derive(Default)]
pub struct TimerManager {
    tick: u64,
    timers: BinaryHeap<Timer>,
}

impl TimerManager {
    pub fn new() -> TimerManager {
        let mut timers = BinaryHeap::new();
        timers.push(Timer::sentinel());
        Self { tick: 0, timers }
    }

    /// ticks the clock and returns whether switching a task is needed or not
    pub fn tick(&mut self, task_manager: &mut TaskManager) -> bool {
        // unsafe { write_volatile(&mut self.tick as *mut u64, self.tick + 1) };
        self.tick += 1;

        let mut task_timer_timeout = false;
        loop {
            let t = self.timers.peek().unwrap();
            if t.timeout > self.tick {
                break;
            }

            if t.value == TASK_TIMER_VALUE {
                task_timer_timeout = true;
                self.timers.pop();
                self.add_timer_for_switching_task(task_manager.main_task().id());
                continue;
            }

            if let Some(task_id) = t.task_id {
                let m = Message::new(MessageType::TimerTimeout {
                    timeout: t.timeout,
                    value: t.value,
                });
                let _ = task_manager.send_message(task_id, m);
            }
            self.timers.pop();
        }

        task_timer_timeout
    }

    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    /// # Safety
    pub unsafe fn current_tick_with_lock(&self) -> u64 {
        asm!("cli");
        let tick = read_volatile(&self.tick as *const u64);
        asm!("sti");
        tick
    }

    pub fn add_timer(&mut self, timer: Timer) {
        self.timers.push(timer);
    }

    pub(crate) fn add_timer_for_switching_task(&mut self, main_task_id: TaskID) {
        self.add_timer(Timer::new(
            self.tick + TASK_TIMER_PERIOD,
            TASK_TIMER_VALUE,
            main_task_id,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskContext;
    use alloc::vec;
    use alloc::vec::Vec;

    #[test]
    fn timer_manager_tick() {
        let mut task_manager = TaskManager::new(dummy_context, |_| {});
        task_manager.initialize(|| 0);
        let main_task_id = task_manager.main_task().id();

        let mut manager = TimerManager::new();
        manager.add_timer(Timer::new(3, 3, main_task_id));
        manager.add_timer(Timer::new(1, 1, main_task_id));
        manager.add_timer(Timer::new(2, 2, main_task_id));
        manager.add_timer(Timer::new(1, 11, main_task_id));

        manager.tick(&mut task_manager);
        assert_eq!(
            get_timers(&mut manager),
            vec![
                Timer::new(u64::MAX, -1, main_task_id),
                Timer::new(3, 3, main_task_id),
                Timer::new(2, 2, main_task_id)
            ]
        );
        assert_eq!(
            get_received_message_timers(&mut task_manager),
            vec![message(1, 1), message(1, 11)]
        );

        manager.tick(&mut task_manager);
        assert_eq!(
            get_timers(&mut manager),
            vec![
                Timer::new(u64::MAX, -1, main_task_id),
                Timer::new(3, 3, main_task_id)
            ]
        );
        assert_eq!(
            get_received_message_timers(&mut task_manager),
            vec![message(2, 2)]
        );

        manager.tick(&mut task_manager);
        assert_eq!(
            get_timers(&mut manager),
            vec![Timer::new(u64::MAX, -1, main_task_id)]
        );
        assert_eq!(
            get_received_message_timers(&mut task_manager),
            vec![message(3, 3)]
        );

        // u64::max Timer should not be timeout event if tick method is called multiple times.
        manager.tick(&mut task_manager);
        manager.tick(&mut task_manager);
        manager.tick(&mut task_manager);
        assert_eq!(
            get_timers(&mut manager),
            vec![Timer::new(u64::MAX, -1, main_task_id)]
        );
        assert_eq!(get_received_message_timers(&mut task_manager), vec![]);
    }

    fn message(timeout: u64, value: i32) -> MessageType {
        MessageType::TimerTimeout { timeout, value }
    }

    fn get_timers(m: &mut TimerManager) -> Vec<Timer> {
        let mut v = m.timers.iter().copied().collect::<Vec<_>>();
        v.sort();
        v
    }

    fn get_received_message_timers(task_manager: &mut TaskManager) -> Vec<MessageType> {
        let task = task_manager.main_task_mut();
        let mut received = vec![];
        while let Some(message) = task.receive_message() {
            received.push(message.m_type);
        }
        received
    }

    unsafe fn dummy_context(_a: &TaskContext, _b: &TaskContext) {}
}
