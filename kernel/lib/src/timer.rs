use core::arch::asm;
use core::ptr::{read_volatile, write_volatile};

const COUNT_MAX: u32 = 0xffffffff;

pub mod global {
    use super::{divide_config, initial_count, lvt_timer, TimerManager};
    use crate::interrupt::InterruptVectorNumber;

    static mut TIMER_MANAGER: TimerManager = TimerManager::new();
    pub fn timer_manager() -> &'static mut TimerManager {
        unsafe { &mut TIMER_MANAGER }
    }

    pub fn initialize_lapic_timer() {
        unsafe {
            divide_config().write_volatile(0b1011);
            lvt_timer().write_volatile((0b010 << 16) | InterruptVectorNumber::LAPICTimer as u32);
            initial_count().write_volatile(0x1000000)
        }
    }

    pub fn lapic_timer_on_interrupt() {
        timer_manager().tick();
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

pub struct TimerManager {
    tick: u64,
}

impl TimerManager {
    pub const fn new() -> TimerManager {
        Self { tick: 0 }
    }

    pub fn tick(&mut self) {
        // unsafe { write_volatile(&mut self.tick as *mut u64, self.tick + 1) };
        self.tick += 1;
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
}
