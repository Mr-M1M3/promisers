// pub mod job;
use std::thread::JoinHandle;
use std::{
    sync::{
        Arc, Mutex,
        mpsc::{Sender, channel},
    },
    thread,
};

pub type Job = Box<dyn FnOnce() + Send + 'static>;
pub struct ThreadPool {
    workers: Vec<JoinHandle<()>>,
    sender: Sender<Job>,
}

impl ThreadPool {
    pub fn with_threads(thread_count: usize) -> Self {
        let (tx, rx) = channel::<Job>();
        let rx = Arc::new(Mutex::new(rx));
        Self {
            sender: tx,
            workers: (0..thread_count)
                .map(|_| {
                    let rx = Arc::clone(&rx);

                    return thread::spawn(move || {
                        loop {
                            let job = match rx.lock().expect("couldn't lock the rx").recv() {
                                Ok(job) => {
                                    // we don't call our job inside match expression
                                    // cause, the mutex guard will be alive for the match body
                                    // and if we call the job inside the body
                                    // mutex guard wouldn't be dropped until the job is done
                                    // and other threads cannot acquire the lock to poll for new tasks
                                    job
                                }
                                Err(_) => {
                                    // rx.recv() only returns error if the transmitter has already hung up
                                    // if the transmitter has hung up, that means, no more work is coming
                                    // so we break the loop
                                    // now join_handle.join() will see that the thread has finished working
                                    break;
                                }
                            };
                            job();
                        }
                    });
                })
                .collect::<Vec<JoinHandle<()>>>(),
        }
    }
    pub fn run<F>(&self, job: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnOnce() + Send + 'static,
    {
        return Ok(self.sender.send(Box::new(job))?);
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        let (replacing_sender, _) = channel();
        let original_sender = std::mem::replace(&mut self.sender, replacing_sender);
        // we need to drop the original sender in order for the .recv() call to not block the thread anymore
        // and spawned thread to finish. otherwise JoinHandle::join() will take forever
        drop(original_sender);
        for worker in std::mem::replace(&mut self.workers, Vec::new()) {
            worker.join().expect("couldn't join spawned threads");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    use super::ThreadPool;

    #[test]
    fn threadpool_spawn_task() {
        let pool = ThreadPool::with_threads(4);

        let timer = Instant::now();
        pool.run(move || {
            thread::sleep(Duration::from_secs(3));
            println!("seconds passed: 3");
        })
        .unwrap();
        pool.run(move || {
            thread::sleep(Duration::from_secs(5));
            println!("seconds passed: 5");
        })
        .unwrap();
        pool.run(move || {
            thread::sleep(Duration::from_secs(6));
            println!("seconds passed: 6");
        })
        .unwrap();
        pool.run(move || {
            thread::sleep(Duration::from_secs(8));
            println!("seconds passed: 8");
        })
        .unwrap();
        pool.run(move || {
            thread::sleep(Duration::from_secs(1));
            println!("seconds passed: 1");
        })
        .unwrap();
        drop(pool);
        let elapsed = timer.elapsed();
        println!("--------------------------elapsed: {:?}", elapsed);
    }
}
