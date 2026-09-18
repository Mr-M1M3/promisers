mod state;
use state::PromiseState;
use std::{cell::RefCell, rc::Rc};

use crate::executor::Executor;

pub struct Promise<T: 'static, E: 'static> {
    state: PromiseState<T, E>,
    on_resolve: Option<Box<dyn FnOnce(T)>>,
    on_reject: Option<Box<dyn FnOnce(E)>>,
    executor: Rc<RefCell<Executor>>,
}

impl<T: 'static, E: 'static> Promise<T, E> {
    pub fn new(
        f: Box<dyn FnOnce(Box<dyn FnOnce(T)>, Box<dyn FnOnce(E)>, Rc<RefCell<Executor>>)>,
        executor: Rc<RefCell<Executor>>,
    ) -> Rc<RefCell<Self>> {

        let internal_promise = Rc::new(RefCell::new(Self {
            state: PromiseState::Pending,
            on_reject: None,
            on_resolve: None,
            executor: Rc::clone(&executor),
        }));
        let internal_promise_mask_1 = Rc::clone(&internal_promise);
        let internal_promise_mask_2 = Rc::clone(&internal_promise);
        f(
            Box::new(move |data: T| {
                Self::resolve(internal_promise_mask_1, data);
            }),
            Box::new(move |err| {
                Self::reject(internal_promise_mask_2, err);
            }),
            Rc::clone(&executor),
        );
        return internal_promise;
    }
    fn resolve(to_resolve: Rc<RefCell<Self>>, data: T) {
        let cb_mask: Option<Box<dyn FnOnce(T)>>;
        {
            cb_mask = to_resolve.borrow_mut().on_resolve.take();
        }
        match cb_mask {
            Some(cb) => {
                cb(data);
                to_resolve.borrow_mut().state = PromiseState::Done;
            }
            None => {
                to_resolve.borrow_mut().state =
                    PromiseState::Resolved(data);
            }
        }
    }

    fn reject(to_reject: Rc<RefCell<Self>>, err: E) {
        let cb_mask: Option<Box<dyn FnOnce(E)>>;
        {
            cb_mask = to_reject.borrow_mut().on_reject.take();
        }
        match cb_mask {
            Some(cb) => {
                cb(err);
                to_reject.borrow_mut().state = PromiseState::Done;
            }
            None => {
                to_reject.borrow_mut().state = PromiseState::Rejected(err);
            }
        }
    }

    pub fn is_pending(&self) -> bool {
        if let PromiseState::Pending = self.state {
            return true;
        } else {
            return false;
        }
    }
    pub fn is_resolved(&self) -> bool {
        if let PromiseState::Resolved(_) = self.state {
            return true;
        } else {
            return false;
        }
    }
    pub fn is_rejected(&self) -> bool {
        if let PromiseState::Rejected(_) = self.state {
            return true;
        } else {
            return false;
        }
    }
    pub fn then(&mut self, cb: Box<dyn FnOnce(T)>) -> &mut Self {
        match std::mem::replace(&mut self.state, PromiseState::Done) {
            // no-op is promise is done
            PromiseState::Done => self,
            // if the promise is already resolved
            PromiseState::Resolved(data) => {
                self.executor.borrow_mut().schedule(Box::new(move || {
                    cb(data);
                }));
                self.state = PromiseState::Done;
                self
            }
            // we changed state in std::mem::replace
            state => {
                self.on_resolve = Some(cb);
                self.state = state;
                self
            }
        }
    }
    pub fn catch(&mut self, cb: Box<dyn FnOnce(E)>) {
        match std::mem::replace(&mut self.state, PromiseState::Done) {
            // no-op is promise is done
            PromiseState::Done => (),
            // if the promise is already resolved
            PromiseState::Rejected(err) => {
                self.executor.borrow_mut().schedule(Box::new(move || {
                    cb(err);
                }));
                self.state = PromiseState::Done;
            }
            // we changed state in std::mem::replace
            state => {
                self.on_reject = Some(cb);
                self.state = state;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn init() {
        let executor = Rc::new(RefCell::new(Executor::new()));
        let executor_mask = Rc::clone(&executor);
        let a: Rc<RefCell<Promise<i32, i32>>> = Promise::new(
            Box::new(|acc, _, e| {
                acc(10);
                Promise::new(
                    Box::new(|_, rej, _| {
                        rej(300);
                    }),
                    e,
                )
                .borrow_mut()
                .then(Box::new(|data: i32| {
                    println!("it worked too, {}", data);
                }))
                .catch(Box::new(|err| {
                    println!("errors works too, {}", err);
                }));
            }),
            executor_mask,
        );
        a.borrow_mut().then(Box::new(|data| {
            println!("LOl, it worked  😐, {}", data);
        }));

        Executor::run(executor);
    }
}
