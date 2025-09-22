// Cooperative scheduler for O3StorageOS
#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::boxed::Box;

/// Simple cooperative scheduler for O3Storage tasks
pub struct Scheduler {
    tasks: Vec<Task>,
    current_task: usize,
}

pub struct Task {
    id: usize,
    state: TaskState,
    function: Box<dyn FnMut() + Send>,
}

#[derive(Debug, Clone, Copy)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Terminated,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            current_task: 0,
        }
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    pub fn run(&mut self) -> ! {
        loop {
            if self.tasks.is_empty() {
                // No tasks, halt
                halt();
            }

            // Find next ready task
            let mut found_task = false;
            for _ in 0..self.tasks.len() {
                if self.current_task >= self.tasks.len() {
                    self.current_task = 0;
                }

                if let Some(task) = self.tasks.get_mut(self.current_task) {
                    if matches!(task.state, TaskState::Ready) {
                        task.state = TaskState::Running;
                        (task.function)();
                        task.state = TaskState::Ready;
                        found_task = true;
                        break;
                    }
                }
                
                self.current_task += 1;
            }

            if !found_task {
                // No ready tasks, yield CPU
                yield_cpu();
            }
        }
    }
}

impl Task {
    pub fn new<F>(function: F) -> Self
    where
        F: FnMut() + Send + 'static,
    {
        static mut TASK_COUNTER: usize = 0;
        let id = unsafe {
            TASK_COUNTER += 1;
            TASK_COUNTER
        };

        Self {
            id,
            state: TaskState::Ready,
            function: Box::new(function),
        }
    }
}

pub fn yield_now() {
    // In a real implementation, this would context switch
    // For now, just a hint to the scheduler
}

fn yield_cpu() {
    // Halt until next interrupt
    unsafe {
        core::arch::asm!("hlt");
    }
}

fn halt() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}