//! Add way to run sequential actions, synchronizing them and improving
//! user experience while awaiting for the tasks to finished.
//!
//! # Example
//!
//! The following example runs 6 tasks, awaiting for each of them to produce
//! an output value.
//!
//! The terminal output will print a spinning ticker, with the alias
//! of each tasks being updated upon completion.
//!
//! ```no_run
//! # use cardano_cli::utils::{term::{Term, Config}, action::{Executor, Async}};
//! use std::time::Duration;
//! use std::thread;
//! # pub fn main() {
//!     let mut term = Term::new(Config::default());
//!
//!     let executor = Executor::new(0u8)
//!          .spawn("return 5: ", |_| { thread::sleep(Duration::new(1, 0)); 5 })
//!          .spawn("add 1: ", |v| { thread::sleep(Duration::new(1, 0)); v + 1 })
//!          .spawn("multiply by 3: ", |v| { thread::sleep(Duration::new(1, 0)); v * 3 })
//!          .spawn("multiply by 2: ", |v| { thread::sleep(Duration::new(1, 0)); v * 2 })
//!          .spawn("divide by 4: ", |v| { thread::sleep(Duration::new(1, 0)); v / 4 })
//!          .spawn("add 3: ", |v| { thread::sleep(Duration::new(1, 0)); v + 3 });
//!
//!     let mut v = None;
//!     {
//!         let mut ticker = term.progress_tick();
//!         loop {
//!             match executor.poll(&mut ticker).unwrap() {
//!                 Async::NotReady => {
//!                 },
//!                 Async::Ready(t) => {
//!                     ticker.end();
//!                     v = Some(t);
//!                     break;
//!                 }
//!             }
//!         }
//!     }
//!
//!     term.info(&format!("done: {:?}", v)).unwrap();
//! # }
//! ```

use std::{time::{Duration}, thread, sync::{Arc, Mutex, Condvar}, mem};
use super::super::utils::term::{Progress};

/// represent an asynchronous result, either the result is Ready or
/// it is not ready.
///
/// This is very similar to `Option<T>` but it has a special meaning.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum Async<T> {
    /// the asynchronous operation finished and here is the result.
    Ready(T),
    /// the asynchronous operation is not finished yet.
    NotReady
}
unsafe impl<T: Send> Send for Async<T> {}
unsafe impl<T: Sync> Sync for Async<T> {}

impl<T> Async<T> {
    /// map the result of the async operation
    pub fn map<F, U>(self, f: F) -> Async<U>
        where F: FnOnce(T) -> U
    {
        match self {
            Async::NotReady => Async::NotReady,
            Async::Ready(t) => Async::Ready(f(t))
        }
    }

    /// tell if the Async is ready to unwrap or not
    pub fn ready(&self) -> bool {
        match self {
            Async::NotReady => false,
            Async::Ready(_) => true
        }
    }
}

#[derive(Debug)]
pub enum Error {

}

/// convenient alias for polling the result of a given operation.
pub type Poll<T> = Result<Async<T>, Error>;

/// sequential operation manager
///
/// Will hold references to the running tasks. They will be executed in
/// the same sequential order as the call to `spawn` function.
///
/// See module documentation for an example.
///
pub struct Executor<T> {
    all_threads: Vec<thread::JoinHandle<()>>,
    current_receiver: Crate<T>,
    current_worker: Crate<String>

}
impl<T: Send + 'static> Executor<T> {

    /// create a new executor manager with the initial value,
    /// the first spawning task will consume this initial value
    /// and will start straight away.
    ///
    pub fn new(initial: T) -> Self {
        let r = Crate::new();
        r.store(initial).unwrap();

        Executor {
            all_threads: Vec::with_capacity(10),
            current_receiver: r,
            current_worker: Crate::new()
        }
    }

    /// spawn a new task. this function append the given task to the sequence of
    /// functions to execute. Consuming the return value of the previously spawn
    /// task (or the initial value is this is the first call to spawn).
    ///
    pub fn spawn<F, Q>(self, alias: &'static str, f: F) -> Executor<Q>
        where F: FnOnce(T) -> Q
            , F: Send + 'static
            , Q: Send + 'static
    {
        let input = self.current_receiver.clone();
        let mut all_threads = self.all_threads;
        let output = Crate::new();
        let current_receiver = output.clone();
        let current_worker = self.current_worker.clone();
        let thread = thread::spawn(move || {
            let res = loop {
                match input.await_result().unwrap() {
                    Async::NotReady => {},
                    Async::Ready(v) => break(v)
                }
            };
            current_worker.store(alias.to_owned()).unwrap();
            output.store(f(res)).unwrap();
        });

        all_threads.push(thread);
        Executor {
            all_threads,
            current_receiver,
            current_worker: self.current_worker
        }
    }

    /// pause the calling thread until the `store` function is called.
    ///
    /// This function will also update the given `ticker` with the currently
    /// working task (see alias parameter of the spawn function) and the
    /// general progress.
    pub fn poll<'a>(&self, ticker: &mut Progress<'a>) -> Poll<T> {
        loop {
            if let Some(r) = self.current_receiver.await_result_timeout(Duration::from_millis(100)).unwrap() {
                return Ok(r);
            } else {
                if let Some(Async::Ready(worker_alias)) = self.current_worker.await_result_timeout(Duration::new(0, 0)).unwrap() {
                    ticker.message(&worker_alias);
                    ticker.advance(1);
                } else {
                    ticker.tick();
                }
            }
        }
    }
}

struct Crate<T>(Arc<(Mutex<Async<T>>, Condvar)>);
impl<T> Crate<T> {
    fn new() -> Self {
        Crate(Arc::new((Mutex::new(Async::NotReady), Condvar::new())))
    }

    fn clone(&self) -> Self {
        Crate(self.0.clone())
    }

    /// store the value in the crate, awakening the awaiting thread (if any)
    fn store(&self, t: T) -> Result<(), Error> {
        let &(ref lock, ref cvar) = &*self.0;
        let mut result = lock.lock().unwrap();

        *result = Async::Ready(t);
        cvar.notify_one();
        Ok(())
    }

    /// pause the calling thread until the `store` function is called.
    fn await_result(&self) -> Poll<T> {
        let &(ref lock, ref cvar) = &*self.0;
        let mut result = lock.lock().unwrap();

        result = if let Async::NotReady = *result {
            cvar.wait(result).unwrap()
        } else {
            result
        };

        let mut val = Async::NotReady;
        mem::swap(&mut *result, &mut val);

        Ok(val)
    }

    /// pause the calling thread until the `store` function is called.
    fn await_result_timeout(&self, dur: Duration) -> Result<Option<Async<T>>, Error> {
        let &(ref lock, ref cvar) = &*self.0;
        let mut result = lock.lock().unwrap();

        result = if let Async::NotReady = *result {
            let r = cvar.wait_timeout(result, dur).unwrap();
            if r.1.timed_out() {
                return Ok(None)
            } else {
                r.0
            }
        } else {
            result
        };

        let mut val = Async::NotReady;
        mem::swap(&mut *result, &mut val);

        Ok(Some(val))
    }
}
