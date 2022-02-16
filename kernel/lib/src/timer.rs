const COUNT_MAX: u32 = 0xffffffff;

pub mod global {
    use super::{divide_config, initial_count, lvt_timer, COUNT_MAX};
    use crate::interrupt::InterruptVectorNumber;

    pub fn initialize_lapic_timer() {
        unsafe {
            divide_config().write_volatile(0b1011);
            lvt_timer().write_volatile((0b010 << 16) | InterruptVectorNumber::LAPICTimer as u32);
            initial_count().write_volatile(COUNT_MAX)
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
