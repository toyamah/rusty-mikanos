use crate::asm::{get_cr3, switch_context};
use crate::error::{Code, Error};
use crate::make_error;
use crate::message::Message;
use crate::segment::{KERNEL_CS, KERNEL_SS};
use alloc::collections::VecDeque;
use alloc::vec;
use alloc::vec::Vec;
use core::mem;

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
}

impl Task {
    const DEFAULT_STACK_BYTES: usize = 4096;
    pub fn new(id: u64) -> Task {
        Self {
            id,
            stack: vec![],
            context: TaskContext::default_(),
            messages: VecDeque::new(),
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
    next_id: u64,
    current_task_index: usize,
    running_task_ids: VecDeque<u64>,
    main_task_id: u64,
}

impl TaskManager {
    pub fn new() -> TaskManager {
        Self {
            tasks: vec![],
            next_id: 0,
            current_task_index: 0,
            running_task_ids: VecDeque::new(),
            main_task_id: 0,
        }
    }

    pub fn initialize_main_task(&mut self) {
        assert!(self.tasks.is_empty());
        self.main_task_id = self.new_task().id;
        self.running_task_ids.push_back(self.main_task_id);
    }

    pub fn new_task(&mut self) -> &mut Task {
        self.tasks.push(Task::new(self.next_id));
        self.next_id += 1;
        self.tasks.iter_mut().last().unwrap()
    }

    pub fn switch_task(&mut self) {
        self._switch_task(false);
    }

    fn _switch_task(&mut self, current_sleep: bool) {
        let current_task_id = self.running_task_ids.pop_front().unwrap();
        let current_task = self.tasks.get(current_task_id as usize).unwrap();
        if !current_sleep {
            self.running_task_ids.push_back(current_task_id);
        }

        let next_task_id = *self.running_task_ids.front().unwrap();
        let next_task = self.tasks.get(next_task_id as usize).unwrap();
        unsafe { switch_context(&next_task.context, &current_task.context) }
    }

    pub fn sleep(&mut self, task_id: u64) -> Result<(), Error> {
        let no_such_task = self.tasks.get(task_id as usize).is_none();
        if no_such_task {
            return Err(make_error!(Code::NoSuchTask));
        }

        let index = self
            .running_task_ids
            .iter()
            .enumerate()
            .find(|(_, &id)| id == task_id)
            .map(|(index, _)| index);

        if let Some(i) = index {
            if i == 0 {
                self.switch_task();
            } else {
                self.running_task_ids.remove(i);
            }
        }
        Ok(())
    }

    pub fn wake_up(&mut self, task_id: u64) -> Result<(), Error> {
        let no_such_task = self.tasks.get(task_id as usize).is_none();
        if no_such_task {
            return Err(make_error!(Code::NoSuchTask));
        }

        let is_not_running = !self.running_task_ids.iter().any(|&id| id == task_id);

        if is_not_running {
            self.running_task_ids.push_back(task_id)
        }
        Ok(())
    }

    pub fn send_message(&mut self, task_id: u64, message: Message) -> Result<(), Error> {
        let task = self.tasks.get_mut(task_id as usize);
        if let Some(task) = task {
            task.send_message(message);
            self.wake_up(task_id).unwrap();
            Ok(())
        } else {
            Err(make_error!(Code::NoSuchTask))
        }
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
