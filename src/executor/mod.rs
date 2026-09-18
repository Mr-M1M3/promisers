use std::{cell::RefCell, collections::VecDeque, rc::Rc};
pub struct Executor {
    queue: VecDeque<Box<dyn FnOnce()>>,
}

impl Executor {
    pub fn new() -> Self {
        Self { queue: VecDeque::new() }
    }

    pub fn schedule(&mut self, f: Box<dyn FnOnce()>) {
        self.queue.push_back(f);
    }
    
    pub fn run(executor: Rc<RefCell<Self>>){
        while let Some(f) = executor.borrow_mut().queue.pop_front() {
            f();
        }
    }
}
