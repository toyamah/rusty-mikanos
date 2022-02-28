use crate::message::{Arg, Message, MessageType, TimerMessage};
use crate::task::TaskManager;
use alloc::collections::BinaryHeap;
use core::arch::asm;
use core::cmp::Ordering;
use core::ptr::read_volatile;

const COUNT_MAX: u32 = 0xffffffff;
pub const TIMER_FREQ: u64 = 100;

pub const TASK_TIMER_PERIOD: u64 = TIMER_FREQ / 50;
pub const TASK_TIMER_VALUE: i32 = i32::MIN;

pub mod global {
    use super::{divide_config, initial_count, lvt_timer, measure_time, TimerManager, TIMER_FREQ};
    use crate::acpi;
    use crate::interrupt::{notify_end_of_interrupt, InterruptVectorNumber};
    use crate::task::TaskManager;

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

    pub fn lapic_timer_on_interrupt(task_manager: &mut TaskManager) {
        let task_timer_timeout = timer_manager().tick(task_manager);
        notify_end_of_interrupt();
        if task_timer_timeout {
            task_manager.switch_task();
        }
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
}

impl Timer {
    pub fn new(timeout: u64, value: i32) -> Timer {
        Self { timeout, value }
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
        timers.push(Timer::new(u64::MAX, -1));
        Self { tick: 0, timers }
    }

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
                self.timers
                    .push(Timer::new(self.tick + TASK_TIMER_PERIOD, TASK_TIMER_VALUE));
                continue;
            }

            let m = Message::new(
                MessageType::TimerTimeout,
                Arg {
                    timer: TimerMessage::new(t.timeout, t.value),
                },
            );
            task_manager
                .send_message(task_manager.main_task().id(), m)
                .unwrap();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::TimerMessage;
    use alloc::vec;
    use alloc::vec::Vec;

    #[test]
    fn timer_manager_tick() {
        let mut manager = TimerManager::new();
        manager.add_timer(Timer::new(3, 3));
        manager.add_timer(Timer::new(1, 1));
        manager.add_timer(Timer::new(2, 2));
        manager.add_timer(Timer::new(1, 11));
        let mut queue = VecDeque::new();

        manager.tick(&mut queue);
        assert_eq!(
            get_timers(&mut manager),
            vec![Timer::new(u64::MAX, -1), Timer::new(3, 3), Timer::new(2, 2)]
        );
        assert_eq!(
            get_message_timers(&mut queue),
            vec![message(1, 1), message(1, 11)]
        );

        manager.tick(&mut queue);
        assert_eq!(
            get_timers(&mut manager),
            vec![Timer::new(u64::MAX, -1), Timer::new(3, 3)]
        );
        assert_eq!(
            get_message_timers(&mut queue),
            vec![message(1, 1), message(1, 11), message(2, 2)]
        );

        manager.tick(&mut queue);
        assert_eq!(get_timers(&mut manager), vec![Timer::new(u64::MAX, -1)]);
        assert_eq!(
            get_message_timers(&mut queue),
            vec![message(1, 1), message(1, 11), message(2, 2), message(3, 3)]
        );

        // u64::max Timer should not be timeout event if tick method is called multiple times.
        manager.tick(&mut queue);
        manager.tick(&mut queue);
        manager.tick(&mut queue);
        assert_eq!(get_timers(&mut manager), vec![Timer::new(u64::MAX, -1)]);
        assert_eq!(
            get_message_timers(&mut queue),
            vec![message(1, 1), message(1, 11), message(2, 2), message(3, 3)]
        );
    }

    fn message(timeout: u64, value: i32) -> TimerMessage {
        TimerMessage::new(timeout, value)
    }

    fn get_timers(m: &mut TimerManager) -> Vec<Timer> {
        let mut v = m.timers.iter().copied().collect::<Vec<_>>();
        v.sort();
        v
    }

    fn get_message_timers(queue: &mut VecDeque<Message>) -> Vec<TimerMessage> {
        queue.iter().map(|m| unsafe { m.arg.timer }).collect()
    }
}
