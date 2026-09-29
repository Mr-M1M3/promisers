mod state;
use state::PromiseState;
use std::{cell::RefCell, rc::Rc};

use crate::keeper::Keeper;

pub struct Promise<T: 'static, E: 'static> {
    state: PromiseState<T, E>,
    on_resolve: Option<Box<dyn FnOnce(T)>>,
    on_reject: Option<Box<dyn FnOnce(E)>>,
    keeper: Rc<RefCell<Keeper>>,
}

impl<T: 'static, E: 'static> Promise<T, E> {
    pub fn new<F>(f: F, keeper: Rc<RefCell<Keeper>>) -> Rc<RefCell<Self>>
    where
        F: FnOnce(Box<dyn FnOnce(T)>, Box<dyn FnOnce(E)>, Rc<RefCell<Keeper>>),
    {
        let internal_promise = Rc::new(RefCell::new(Self {
            state: PromiseState::Pending,
            on_reject: None,
            on_resolve: None,
            keeper: Rc::clone(&keeper),
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
            Rc::clone(&keeper),
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
                to_resolve.borrow_mut().state = PromiseState::Resolved(data);
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
    pub fn then<F: FnOnce(T) + 'static>(&mut self, cb: F) -> &mut Self {
        match std::mem::replace(&mut self.state, PromiseState::Done) {
            // no-op is promise is done
            PromiseState::Done => self,
            // if the promise is already resolved
            PromiseState::Resolved(data) => {
                self.keeper.borrow_mut().schedule_immediate(move || {
                    cb(data);
                });
                self.state = PromiseState::Done;
                self
            }
            // we changed state in std::mem::replace
            state => {
                self.on_resolve = Some(Box::new(cb));
                self.state = state;
                self
            }
        }
    }
    pub fn catch<F: FnOnce(E) + 'static>(&mut self, cb: F) {
        match std::mem::replace(&mut self.state, PromiseState::Done) {
            // no-op is promise is done
            PromiseState::Done => (),
            // if the promise is already resolved
            PromiseState::Rejected(err) => {
                self.keeper.borrow_mut().schedule_immediate(move || {
                    cb(err);
                });
                self.state = PromiseState::Done;
            }
            // we changed state in std::mem::replace
            state => {
                self.on_reject = Some(Box::new(cb));
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
        let keeper = Rc::new(RefCell::new(Keeper::with_threads(1)));
        let keeper_mask = Rc::clone(&keeper);
        let a: Rc<RefCell<Promise<i32, i32>>> = Promise::new(
            |acc, _, e| {
                acc(10);
                Promise::new(
                    |_, rej, _| {
                        rej(300);
                    },
                    e,
                )
                .borrow_mut()
                .then(|data: i32| {
                    println!("it worked too, {}", data);
                })
                .catch(|err| {
                    println!("errors works too, {}", err);
                });
            },
            keeper_mask,
        );
        a.borrow_mut().then(|data| {
            println!("LOl, it worked  😐, {}", data);
        });
        keeper.borrow_mut().keep();
    }
}
