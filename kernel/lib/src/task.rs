use crate::asm::{get_cr3, switch_context};
use crate::error::{Code, Error};
use crate::make_error;
use crate::message::Message;
use crate::segment::{KERNEL_CS, KERNEL_SS};
use alloc::collections::VecDeque;
use alloc::vec;
use alloc::vec::Vec;
use core::mem;
use core::ops::Not;

pub mod global {
    use crate::task::TaskManager;
    use crate::timer::global::timer_manager;
    use crate::timer::{Timer, TASK_TIMER_PERIOD, TASK_TIMER_VALUE};
    use core::arch::asm;

    static mut TASK_MANAGER: Option<TaskManager> = None;
    pub fn task_manager() -> &'static mut TaskManager {
        unsafe { TASK_MANAGER.as_mut().unwrap() }
    }

    pub fn initialize() {
        unsafe { TASK_MANAGER = Some(TaskManager::new()) };
        task_manager().initialize_main_task();

        unsafe { asm!("cli") };
        timer_manager().add_timer(Timer::new(
            timer_manager().current_tick() + TASK_TIMER_PERIOD,
            TASK_TIMER_VALUE,
        ));
        unsafe { asm!("sti") };
    }
}

pub struct Task {
    id: u64,
    stack: Vec<u64>,
    context: TaskContext,
    messages: VecDeque<Message>,
    pub level: PriorityLevel,
    pub is_running: bool,
}

impl Task {
    const DEFAULT_STACK_BYTES: usize = 4096;
    pub fn new(id: u64, level: PriorityLevel) -> Task {
        Self {
            id,
            stack: vec![],
            context: TaskContext::default_(),
            messages: VecDeque::new(),
            level,
            is_running: false,
        }
    }

    pub fn init_context(&mut self, task_func: fn(u64, usize) -> (), data: u64) -> &mut Task {
        let stack_size = Task::DEFAULT_STACK_BYTES / mem::size_of::<u64>();
        self.stack.resize(stack_size, 0);
        let stack_end = self.stack.last().unwrap() as *const _ as u64;

        let context = &mut self.context;
        context.cr3 = get_cr3();
        context.rflags = 0x202;
        context.cs = KERNEL_CS as u64;
        context.ss = KERNEL_SS as u64;
        context.rsp = (stack_end & !0xf) - 8;

        context.rip = task_func as usize as u64;
        context.rdi = self.id;
        context.rsi = data;

        context.fxsave_area[24..][..4].copy_from_slice(&0x1f80u32.to_le_bytes());
        self
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn set_level(&mut self, level: PriorityLevel) -> &mut Task {
        self.level = level;
        self
    }

    pub fn set_is_running(&mut self, is_running: bool) -> &mut Task {
        self.is_running = is_running;
        self
    }

    /// needs to call `wake_up` after this method is invoked
    fn send_message(&mut self, message: Message) {
        self.messages.push_back(message)
    }

    pub fn receive_message(&mut self) -> Option<Message> {
        self.messages.pop_front()
    }
}

pub struct TaskManager {
    tasks: Vec<Task>,
    current_task_index: usize,
    next_id: u64,
    main_task_id: u64,
    running_task_ids: [VecDeque<u64>; PriorityLevel::MAX.to_usize() + 1],
    current_level: PriorityLevel,
    level_changed: bool,
}

impl TaskManager {
    #[allow(clippy::new_without_default)]
    pub fn new() -> TaskManager {
        Self {
            tasks: vec![],
            next_id: 0,
            current_task_index: 0,
            main_task_id: 0,
            running_task_ids: Default::default(),
            current_level: PriorityLevel::MAX,
            level_changed: false,
        }
    }

    pub(crate) fn initialize_main_task(&mut self) {
        assert!(self.tasks.is_empty());
        let level = self.current_level;
        let task_id = self.new_task().set_is_running(true).set_level(level).id;
        self.main_task_id = task_id;
        self.running_task_ids_mut(level).push_back(task_id);
    }

    pub fn new_task(&mut self) -> &mut Task {
        self.tasks
            .push(Task::new(self.next_id, PriorityLevel::default()));
        self.next_id += 1;
        self.tasks.iter_mut().last().unwrap()
    }

    pub fn switch_task(&mut self) {
        self._switch_task(false);
    }

    fn _switch_task(&mut self, current_sleep: bool) {
        let current_task_id = self.current_running_task_ids_mut().pop_front().unwrap();
        if !current_sleep {
            self.current_running_task_ids_mut()
                .push_back(current_task_id);
        }
        if self.current_running_task_ids_mut().is_empty() {
            self.level_changed = true;
        }

        if self.level_changed {
            self.level_changed = false;
            for level in (PriorityLevel::MIN.to_usize()..=PriorityLevel::MAX.to_usize()).rev() {
                if self.running_task_ids[level].is_empty().not() {
                    self.current_level = PriorityLevel::new(level as u8);
                    break;
                }
            }
        }

        let next_task_id = *self.current_running_task_ids_mut().front().unwrap();
        let next_task = self.tasks.get(next_task_id as usize).unwrap();
        let current_task = self.tasks.get(current_task_id as usize).unwrap();
        unsafe { switch_context(&next_task.context, &current_task.context) }
    }

    fn change_level_running(&mut self, task_id: u64, level: PriorityLevel) {
        let task_level = self.tasks.get(task_id as usize).unwrap().level;
        if level == task_level {
            return;
        }

        let running_id = *self
            .current_running_task_ids_mut()
            .front()
            .unwrap_or(&(task_id + 1));
        if task_id != running_id {
            // change level of other task
            self.running_task_ids_mut(task_level)
                .remove(task_id as usize);
            self.running_task_ids_mut(level).push_back(task_id);
            self.tasks[task_id as usize].level = level;

            if level > self.current_level {
                self.level_changed = true;
            }
            return;
        }

        // change level myself
        self.current_running_task_ids_mut().pop_front().unwrap();
        self.running_task_ids_mut(level).push_front(task_id);
        self.tasks[task_id as usize].level = level;
        self.current_level = level;
        if level < self.current_level {
            self.level_changed = true;
        }
    }

    pub fn sleep(&mut self, task_id: u64) -> Result<(), Error> {
        let task = self.tasks.get_mut(task_id as usize);
        if task.is_none() {
            return Err(make_error!(Code::NoSuchTask));
        }
        let task = task.unwrap();
        if !task.is_running {
            return Ok(()); // the task has already slept.
        }
        task.is_running = false;

        let is_target_task_running = self
            .current_running_task_ids_mut()
            .front()
            .map(|&index| index == task_id)
            .unwrap_or(false);
        if is_target_task_running {
            self._switch_task(true);
        } else {
            self.current_running_task_ids_mut().remove(task_id as usize);
        }
        Ok(())
    }

    fn current_running_task_ids_mut(&mut self) -> &mut VecDeque<u64> {
        self.running_task_ids_mut(self.current_level)
    }

    fn running_task_ids_mut(&mut self, level: PriorityLevel) -> &mut VecDeque<u64> {
        self.running_task_ids.get_mut(level.to_usize()).unwrap()
    }

    pub fn wake_up(&mut self, task_id: u64) -> Result<(), Error> {
        let index = task_id as usize;
        if self.tasks.get(index).is_none() {
            return Err(make_error!(Code::NoSuchTask));
        }

        let level = self.tasks[index].level;

        if self.tasks[index].is_running {
            self.change_level_running(task_id, level);
            return Ok(());
        }

        self.tasks[index].set_level(level).set_is_running(true);
        self.running_task_ids_mut(level).push_back(task_id);
        if level > self.current_level {
            self.level_changed = true;
        }
        Ok(())
    }

    pub fn send_message(&mut self, task_id: u64, message: Message) -> Result<(), Error> {
        let index = task_id as usize;
        if self.tasks.get(index).is_none() {
            return Err(make_error!(Code::NoSuchTask));
        }

        self.tasks[index].send_message(message);
        self.wake_up(task_id).unwrap();
        Ok(())
    }

    pub fn main_task(&self) -> &Task {
        self.tasks
            .get(self.main_task_id as usize)
            .expect("tasks do not contain main task")
    }

    pub fn main_task_mut(&mut self) -> &mut Task {
        self.tasks
            .get_mut(self.main_task_id as usize)
            .expect("tasks do not contain main task")
    }
}

#[derive(Debug)]
#[repr(C, align(16))]
pub struct TaskContext {
    // offset : 0x00
    cr3: u64,
    rip: u64,
    rflags: u64,
    reserved1: u64,
    // offset : 0x20
    cs: u64,
    ss: u64,
    fs: u64,
    gs: u64,
    // offset : 0x40
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rdi: u64,
    rsi: u64,
    rsp: u64,
    rbp: u64,
    // offset : 0x80
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    // offset : 0xc0
    fxsave_area: [u8; 512],
}

impl TaskContext {
    const fn default_() -> Self {
        Self {
            cr3: 0,
            rip: 0,
            rflags: 0,
            reserved1: 0,
            cs: 0,
            ss: 0,
            fs: 0,
            gs: 0,
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rdi: 0,
            rsi: 0,
            rsp: 0,
            rbp: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            fxsave_area: [0; 512],
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd)]
pub struct PriorityLevel(i8);

impl PriorityLevel {
    const MAX: PriorityLevel = PriorityLevel(3);
    const MIN: PriorityLevel = PriorityLevel(0);

    pub fn new(level: u8) -> Self {
        let level = level as i8;
        assert!(Self::MIN.0 <= level && level <= Self::MAX.0);
        Self(level as i8)
    }

    pub const fn to_usize(&self) -> usize {
        self.0 as usize
    }
}

impl Default for PriorityLevel {
    fn default() -> Self {
        PriorityLevel::new(1)
    }
}
