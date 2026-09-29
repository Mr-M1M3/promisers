mod registry;
use mio::{
    Events, Poll, Token, Waker,
    event::{self, Source},
};
use std::{
    collections::{HashMap, VecDeque},
    io::Error as IoError,
    sync::Arc,
};

use crate::threadpool::ThreadPool;
pub struct Keeper {
    poller: Poll,
    immediately_execute: VecDeque<Box<dyn FnOnce()>>,
    ready_events: event::Events,
    callback_map: HashMap<Token, Box<dyn FnOnce()>>,
    // because, some async task will actually be done on a threadpool
    // we need to keep the waker awake, or else,
    // waker could fail
    // so, we wrap the waker inside an Arc ( to move around threads )
    // and store it here to keep the wakers alive
    waker_map: HashMap<Token, Arc<Waker>>,
    threadpool: ThreadPool,
    latest_waker_id: usize,
}

impl Keeper {
    pub fn with_threads(thread_count: usize) -> Self {
        assert!(thread_count > 0);
        Self {
            immediately_execute: VecDeque::new(),
            ready_events: Events::with_capacity(128),
            poller: Poll::new()
                .expect("couldn't instantiate the poller. report this error on github"),
            callback_map: HashMap::new(),
            waker_map: HashMap::new(),
            threadpool: ThreadPool::with_threads(thread_count),
            latest_waker_id: 0,
        }
    }
    pub fn threadpool(&self) -> &ThreadPool {
        &self.threadpool
    }
    pub fn schedule_immediate(&mut self, f: Box<dyn FnOnce()>) {
        self.immediately_execute.push_back(f);
    }
    pub fn register<'a, T: Source + ?Sized>(
        &mut self,
        registry: registry::Registry<T>,
        cb: Box<dyn FnOnce()>,
    ) -> Result<(), IoError> {
        self.callback_map.insert(registry.id, cb);
        self.poller
            .registry()
            .register(registry.fd, registry.id, registry.interest)
    }

    pub(crate) fn reg_waker(&mut self, cb: Box<dyn FnOnce()>) -> Result<Arc<Waker>, IoError> {
        let id = self.latest_waker_id;
        self.latest_waker_id += 1;
        let token = Token(id);
        self.callback_map.insert(token, cb);
        match Waker::new(self.poller.registry(), token) {
            Ok(waker) => {
                let waker = Arc::new(waker);
                self.waker_map.insert(Token(id), waker.clone());
                return Ok(waker.clone());
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    pub fn keep(&mut self) {
        while let Some(f) = self.immediately_execute.pop_front() {
            f();
        }
        self.immediately_execute.clear();
        let mut ready_events = std::mem::replace(&mut self.ready_events, Events::with_capacity(0));
        loop {
            if self.callback_map.is_empty() {
                break;
            }
            self.poller
                .poll(&mut ready_events, None)
                .expect("you don't want to see this message");
            for event in ready_events.iter() {
                println!("got token: {:?}", event.token());
                let to_call = self
                    .callback_map
                    .remove(&event.token())
                    .expect("no callback for the occurred event wasn't registered. this error shouldn't happen. report this on github");
                self.waker_map.remove(&event.token());
                to_call();
            }
        }
    }
}
