use crate::error::{Code, Error};
use crate::fat::FileDescriptor;
use crate::make_error;
use crate::message::Message;
use crate::segment::{KERNEL_CS, KERNEL_SS};
use alloc::collections::VecDeque;
use alloc::vec;
use alloc::vec::Vec;
use core::arch::asm;
use core::mem;
use core::ops::{Add, AddAssign, Not};

pub mod global {
    use crate::asm::global::{get_cr3, restore_context, switch_context};
    use crate::task::TaskManager;
    use crate::timer::global::timer_manager;
    use core::arch::asm;

    static mut TASK_MANAGER: Option<TaskManager> = None;
    pub fn task_manager() -> &'static mut TaskManager {
        unsafe { TASK_MANAGER.as_mut().unwrap() }
    }

    pub fn initialize() {
        unsafe { TASK_MANAGER = Some(TaskManager::new(switch_context, restore_context)) };
        task_manager().initialize(get_cr3);

        unsafe { asm!("cli") };
        timer_manager().add_timer_for_switching_task(task_manager().main_task_id);
        unsafe { asm!("sti") };
    }

    #[no_mangle]
    extern "C" fn GetCurrentTaskOSStackPointerInRust() -> u64 {
        let p = *task_manager().current_task().os_stack_pointer();
        assert_ne!(p, 1); // Do the same check as c++ code
        p
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TaskID(u64);

impl TaskID {
    pub fn new(v: u64) -> Self {
        Self(v)
    }

    fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

impl Add for TaskID {
    type Output = TaskID;

    fn add(self, rhs: Self) -> Self::Output {
        TaskID(self.0 + rhs.0)
    }
}

impl AddAssign for TaskID {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

pub struct Task {
    id: TaskID,
    stack: Vec<TaskID>,
    context: TaskContext,
    os_stack_ptr: u64,
    messages: VecDeque<Message>,
    level: PriorityLevel,
    is_running: bool,
    files: Vec<Option<FileDescriptor>>,
}

impl Task {
    const DEFAULT_STACK_BYTES: usize = 4096;
    fn new(id: TaskID, level: PriorityLevel) -> Task {
        Self {
            id,
            stack: vec![],
            context: TaskContext::default_(),
            os_stack_ptr: 0,
            messages: VecDeque::new(),
            level,
            is_running: false,
            files: vec![],
        }
    }

    pub fn os_stack_pointer(&self) -> &u64 {
        &self.os_stack_ptr
    }

    pub fn init_context(
        &mut self,
        task_func: fn(u64, usize) -> (),
        data: u64,
        get_cr3: fn() -> u64,
    ) -> &mut Task {
        let stack_size = Task::DEFAULT_STACK_BYTES / mem::size_of::<u64>();
        self.stack.resize(stack_size, TaskID(0));
        let stack_end = self.stack.last().unwrap() as *const _ as u64;

        let context = &mut self.context;
        context.cr3 = get_cr3();
        context.rflags = 0x202;
        context.cs = KERNEL_CS as u64;
        context.ss = KERNEL_SS as u64;
        context.rsp = (stack_end & !0xf) - 8;

        context.rip = task_func as usize as u64;
        context.rdi = self.id.0;
        context.rsi = data;

        context.fxsave_area[24..][..4].copy_from_slice(&0x1f80u32.to_le_bytes());
        self
    }

    pub fn id(&self) -> TaskID {
        self.id
    }

    fn set_level(&mut self, level: PriorityLevel) -> &mut Task {
        self.level = level;
        self
    }

    fn set_is_running(&mut self, is_running: bool) -> &mut Task {
        self.is_running = is_running;
        self
    }

    pub(crate) fn set_cr3(&mut self, cr3: u64) {
        self.context.cr3 = cr3;
    }

    pub(crate) fn get_cr3(&self) -> u64 {
        self.context.cr3
    }

    pub(crate) fn get_files_slice(&self) -> &[Option<FileDescriptor>] {
        self.files.as_slice()
    }

    pub(crate) fn get_files_mut(&mut self) -> &mut Vec<Option<FileDescriptor>> {
        &mut self.files
    }

    pub(crate) fn register_file_descriptor(&mut self, fd: FileDescriptor) -> usize {
        let first_empty = self.files.iter().enumerate().find(|(_, fd)| fd.is_none());

        if let Some((first_empty_index, _)) = first_empty {
            self.files[first_empty_index] = Some(fd);
            first_empty_index
        } else {
            self.files.push(Some(fd));
            self.files.len() - 1
        }
    }

    /// needs to call `wake_up` after this method is invoked
    fn send_message(&mut self, message: Message) {
        self.messages.push_back(message)
    }

    pub fn receive_message(&mut self) -> Option<Message> {
        self.messages.pop_front()
    }

    fn idle(_task_id: u64, _data: usize) {
        loop {
            unsafe { asm!("hlt") };
        }
    }
}

pub struct TaskManager {
    tasks: Vec<Task>,
    next_id: TaskID,
    main_task_id: TaskID,
    running_task_ids: [VecDeque<TaskID>; PriorityLevel::MAX.as_usize() + 1],
    current_level: PriorityLevel,
    level_changed: bool,
    switch_context: unsafe fn(next_ctx: &TaskContext, current_ctx: &TaskContext),
    restore_context: unsafe fn(task_ctx: &TaskContext),
}

impl TaskManager {
    #[allow(clippy::new_without_default)]
    pub fn new(
        switch_context: unsafe fn(next_ctx: &TaskContext, current_ctx: &TaskContext),
        restore_context: unsafe fn(task_ctx: &TaskContext),
    ) -> TaskManager {
        Self {
            tasks: vec![],
            next_id: TaskID(0),
            main_task_id: TaskID(0),
            running_task_ids: Default::default(),
            current_level: PriorityLevel::MAX,
            level_changed: false,
            switch_context,
            restore_context,
        }
    }

    pub(crate) fn initialize(&mut self, get_cr3: fn() -> u64) {
        assert!(self.tasks.is_empty());
        let level = self.current_level;
        let main_task_id = self.new_task().set_is_running(true).set_level(level).id;
        self.main_task_id = main_task_id;
        self.running_task_ids_mut(level).push_back(main_task_id);

        let idle_task_id = self
            .new_task()
            .init_context(Task::idle, 0, get_cr3)
            .set_is_running(true)
            .set_level(PriorityLevel::IDLE)
            .id;
        self.running_task_ids_mut(PriorityLevel::IDLE)
            .push_back(idle_task_id);
    }

    pub fn new_task(&mut self) -> &mut Task {
        self.tasks
            .push(Task::new(self.next_id, PriorityLevel::default()));
        self.next_id += TaskID(1);
        self.tasks.iter_mut().last().unwrap()
    }

    pub fn get_task(&self, task_id: TaskID) -> Option<&Task> {
        self.tasks.get(task_id.as_usize())
    }

    pub fn get_task_mut(&mut self, task_id: TaskID) -> Option<&mut Task> {
        self.tasks.get_mut(task_id.as_usize())
    }

    pub fn current_task(&self) -> &Task {
        let task_id = self
            .current_running_task_ids()
            .front()
            .expect("no such task id");
        let task_id = *task_id;
        self.tasks.get(task_id.as_usize()).expect("no such task")
    }

    pub fn current_task_mut(&mut self) -> &mut Task {
        let task_id = self
            .current_running_task_ids_mut()
            .front()
            .expect("no such task id");
        let task_id = *task_id;
        self.tasks
            .get_mut(task_id.as_usize())
            .expect("no such task")
    }

    pub fn main_task(&self) -> &Task {
        self.tasks
            .get(self.main_task_id.as_usize())
            .expect("tasks do not contain main task")
    }

    pub fn main_task_mut(&mut self) -> &mut Task {
        self.tasks
            .get_mut(self.main_task_id.as_usize())
            .expect("tasks do not contain main task")
    }

    pub fn switch_task(&mut self, current_ctx: &TaskContext) {
        self.current_task_mut().context.clone_from(current_ctx);

        let current_task_id = self.rotate_current_run_queue(false);

        if self.current_task().id != current_task_id {
            unsafe { (self.restore_context)(&self.current_task().context) }
        }
    }

    pub fn sleep(&mut self, task_id: TaskID) -> Result<(), Error> {
        let task = self.tasks.get_mut(task_id.as_usize());
        if task.is_none() {
            return Err(make_error!(Code::NoSuchTask));
        }
        let task = task.unwrap();
        if !task.is_running {
            return Ok(()); // the task has already slept.
        }
        task.is_running = false;

        let level = task.level;
        let is_target_task_running = self
            .current_running_task_ids_mut()
            .front()
            .map(|&index| index == task_id)
            .unwrap_or(false);
        if is_target_task_running {
            let current_task_id = self.rotate_current_run_queue(true);
            unsafe {
                (self.switch_context)(
                    &self.current_task().context,
                    &self.get_task(current_task_id).unwrap().context,
                )
            };
        } else {
            erase_task_id(self.running_task_ids_mut(level), task_id);
        }
        Ok(())
    }

    pub fn wake_up(&mut self, task_id: TaskID) -> Result<(), Error> {
        let index = task_id.as_usize();
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

    pub fn send_message(&mut self, task_id: TaskID, message: Message) -> Result<(), Error> {
        let index = task_id.as_usize();
        if self.tasks.get(index).is_none() {
            return Err(make_error!(Code::NoSuchTask));
        }

        self.tasks[index].send_message(message);
        self.wake_up(task_id).unwrap();
        Ok(())
    }

    pub fn rotate_current_run_queue(&mut self, current_sleep: bool) -> TaskID {
        let current_task_id = self
            .current_running_task_ids_mut()
            .pop_front()
            .expect("could not pop front");
        if !current_sleep {
            self.current_running_task_ids_mut()
                .push_back(current_task_id);
        }

        if self.current_running_task_ids_mut().is_empty() {
            self.level_changed = true;
        }

        if self.level_changed {
            self.level_changed = false;

            for level in (PriorityLevel::MIN.as_usize()..=PriorityLevel::MAX.as_usize()).rev() {
                if self.running_task_ids[level].is_empty().not() {
                    self.current_level = PriorityLevel::new(level as u8);
                    break;
                }
            }
        }

        current_task_id
    }

    fn current_running_task_ids(&self) -> &VecDeque<TaskID> {
        self.running_task_ids(self.current_level)
    }

    fn current_running_task_ids_mut(&mut self) -> &mut VecDeque<TaskID> {
        self.running_task_ids_mut(self.current_level)
    }

    fn running_task_ids(&self, level: PriorityLevel) -> &VecDeque<TaskID> {
        self.running_task_ids.get(level.as_usize()).unwrap()
    }

    fn running_task_ids_mut(&mut self, level: PriorityLevel) -> &mut VecDeque<TaskID> {
        self.running_task_ids.get_mut(level.as_usize()).unwrap()
    }

    fn change_level_running(&mut self, task_id: TaskID, level: PriorityLevel) {
        let task_level = self.tasks.get(task_id.as_usize()).unwrap().level;
        if level == task_level {
            return;
        }

        let running_id = *self
            .current_running_task_ids_mut()
            .front()
            .unwrap_or(&(task_id + TaskID(1)));
        if task_id != running_id {
            // change level of other task
            erase_task_id(self.running_task_ids_mut(task_level), task_id);
            self.running_task_ids_mut(level).push_back(task_id);
            self.tasks[task_id.as_usize()].level = level;

            if level > self.current_level {
                self.level_changed = true;
            }
            return;
        }

        // change level myself
        self.current_running_task_ids_mut().pop_front().unwrap();
        self.running_task_ids_mut(level).push_front(task_id);
        self.tasks[task_id.as_usize()].level = level;
        self.current_level = level;
        if level < self.current_level {
            self.level_changed = true;
        }
    }
}

fn erase_task_id(queue: &mut VecDeque<TaskID>, target: TaskID) {
    let (index, _) = queue
        .iter()
        .enumerate()
        .find(|(_, &id)| id == target)
        .expect("no such task to be removed");
    queue.remove(index).unwrap();
}

#[derive(Debug, Clone)]
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
    const IDLE: PriorityLevel = Self::MIN;

    pub fn new(level: u8) -> Self {
        let level = level as i8;
        assert!(Self::MIN.0 <= level && level <= Self::MAX.0);
        Self(level as i8)
    }

    pub const fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

impl Default for PriorityLevel {
    fn default() -> Self {
        PriorityLevel::new(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDLE_TASK_ID: TaskID = TaskID(1);

    #[test]
    fn task_manager_sleep_running_task_id() {
        let mut tm = TaskManager::new(|_, _| {}, |_| {});
        tm.initialize(|| 0);

        let t1_id = tm.new_task().set_level(PriorityLevel::MAX).id;
        let t2_id = tm.new_task().set_level(PriorityLevel::new(1)).id;
        let t3_id = tm.new_task().set_level(PriorityLevel::new(1)).id;
        tm.wake_up(t1_id).unwrap();
        tm.wake_up(t2_id).unwrap();
        tm.wake_up(t3_id).unwrap();

        // when the current running id is given,
        tm.sleep(tm.main_task_id).unwrap();

        // then current level should not be changed
        assert_eq!(tm.current_level, PriorityLevel::MAX);

        // the id should be removed from running_task_ids
        assert_eq!(
            tm.running_task_ids,
            [
                VecDeque::from([IDLE_TASK_ID]),
                VecDeque::from([t2_id, t3_id]),
                VecDeque::from([]),
                VecDeque::from([t1_id]),
            ]
        );

        // the task should be changed to sleep
        assert_eq!(tm.tasks[tm.main_task_id.as_usize()].is_running, false);
    }

    #[test]
    fn task_manager_sleep_last_task_at_running_level() {
        let mut tm = TaskManager::new(|_, _| {}, |_| {});
        tm.initialize(|| 0);

        let t1_id = tm.new_task().set_level(PriorityLevel::new(1)).id;
        let t2_id = tm.new_task().set_level(PriorityLevel::new(1)).id;
        tm.wake_up(t1_id).unwrap();
        tm.wake_up(t2_id).unwrap();

        // when the current running id is given,
        tm.sleep(tm.main_task_id).unwrap();

        // then current level should be changed to the next highest level
        assert_eq!(tm.current_level, PriorityLevel::new(1));

        // the id should be removed from running_task_ids
        assert_eq!(
            tm.running_task_ids,
            [
                VecDeque::from([IDLE_TASK_ID]),
                VecDeque::from([t1_id, t2_id]),
                VecDeque::from([]),
                VecDeque::from([]),
            ]
        );

        // the task should be changed to sleep
        assert_eq!(tm.tasks[tm.main_task_id.as_usize()].is_running, false);
    }

    #[test]
    fn task_manager_sleep_not_running_but_same_level() {
        let mut tm = TaskManager::new(|_, _| {}, |_| {});
        tm.initialize(|| 0);

        let t1_id = tm.new_task().set_level(PriorityLevel::MAX).id;
        let t2_id = tm.new_task().set_level(PriorityLevel::new(1)).id;
        let t3_id = tm.new_task().set_level(PriorityLevel::new(1)).id;
        tm.wake_up(t1_id).unwrap();
        tm.wake_up(t2_id).unwrap();
        tm.wake_up(t3_id).unwrap();

        tm.sleep(t1_id).unwrap();

        // current level should not be changed
        assert_eq!(tm.current_level, PriorityLevel::MAX);

        // the id should be removed from running_task_ids
        assert_eq!(
            tm.running_task_ids,
            [
                VecDeque::from([IDLE_TASK_ID]),
                VecDeque::from([t2_id, t3_id]),
                VecDeque::from([]),
                VecDeque::from([tm.main_task_id]),
            ]
        );

        // the running task should still be running
        assert_eq!(tm.tasks[tm.main_task_id.as_usize()].is_running, true);

        // the specified task should be sleep
        assert_eq!(tm.tasks[t1_id.as_usize()].is_running, false);
    }

    #[test]
    fn task_manager_sleep_level_different_from_current_level() {
        let mut tm = TaskManager::new(|_, _| {}, |_| {});
        tm.initialize(|| 0);

        let t1_id = tm.new_task().set_level(PriorityLevel::MAX).id;
        let t2_id = tm.new_task().set_level(PriorityLevel::new(1)).id;
        let t3_id = tm.new_task().set_level(PriorityLevel::new(1)).id;
        tm.wake_up(t1_id).unwrap();
        tm.wake_up(t2_id).unwrap();
        tm.wake_up(t3_id).unwrap();

        // when an id of a task whose level is not the same as current_running_level,
        tm.sleep(t1_id).unwrap();

        // then current level should not be changed
        assert_eq!(tm.current_level, PriorityLevel::MAX);

        // the id should be removed from running_task_ids
        assert_eq!(
            tm.running_task_ids,
            [
                VecDeque::from([IDLE_TASK_ID]),
                VecDeque::from([t2_id, t3_id]),
                VecDeque::from([]),
                VecDeque::from([tm.main_task_id]),
            ]
        );

        // the running task should still be running
        assert_eq!(tm.tasks[tm.main_task_id.as_usize()].is_running, true);

        // the task specified by the arg should be sleep
        assert_eq!(tm.tasks[t1_id.as_usize()].is_running, false);
    }
}
