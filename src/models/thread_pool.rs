use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    },
    thread,
};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

type Job = Box<dyn FnBox + Send + 'static>;

pub struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let message = receiver.lock().unwrap().recv().unwrap();

                match message {
                    Message::NewJob(job) => {
                        job.call_box();
                    }
                    Message::Terminate => {
                        println!("Worker to terminate");
                        break;
                    }
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

impl ThreadPool {
    /// Creates a new threadpool
    /// The num_threads value is the number of threads in the pool
    ///
    /// # Panics
    ///
    /// The `new` function panics if ```num_threads == 0```
    pub fn new(num_workers: usize) -> Self {
        assert!(num_workers > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(num_workers);

        for id in 0..num_workers {
            //Create threads and store them in a vector
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }
        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(Message::NewJob(job)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Sending terminate message to all workers");

        for _ in &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }
        println!("Sutting down all workers");

        for worker in &mut self.workers {
            println!("Shutting down worker");

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

trait FnBox {
    fn call_box(self: Box<Self>);
}
impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<Self>) {
        (*self)()
    }
}

enum Message {
    NewJob(Job),
    Terminate,
}
