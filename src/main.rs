use std::collections::hash_map::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub enum Msg {
    Data { producer: i32, value: i32 } ,
    Done { producer: i32 },
    Shutdown,
}

#[derive(Debug, Default)]
pub struct Data {
    sum: i32,
    closed: bool,
}

fn main() {
    let _ = run_system(3, 1, 5, false);
}

fn run_system(
    num_producers: i32,
    num_consumers: i32,
    msgs_per_producer: i32,
    inject_early_done: bool,
) -> HashMap<i32, Data> {
    let (tx, rx) = mpsc::channel();
    let shared_map: Arc<Mutex<HashMap<i32, Data>>> = Arc::new(Mutex::new(HashMap::new()));


    let mut consumers = Vec::new();

    
    // Consumer
    let mut c_handles: Vec<thread::JoinHandle<()>> = Vec::new();
    for id in 0..num_consumers {
        let (worker_tx, worker_rx) = mpsc::channel();
        consumers.push(worker_tx);

        let map = shared_map.clone();
        c_handles.push(thread::spawn(move || {
            consumer_loop(id, worker_rx, map);
        }));
    }

    // dispatcher
    let dispatcher_handle = thread::spawn(move || {
        for msg in rx {
            match msg {
                Msg::Data{ producer, value: _ } => consumers[producer as usize].send(msg).unwrap(),
                Msg::Done{ producer } => consumers[producer as usize].send(msg).unwrap(),
                Msg::Shutdown => (),
            }
        }

        for tx in consumers {
            let _ = tx.send(Msg::Shutdown);
        }
    });

    // producer
    let mut p_handles: Vec<thread::JoinHandle<()>> = Vec::new();
    for i in 0..num_producers {
        let txc = tx.clone();
        p_handles.push(spawn_producer(i, msgs_per_producer, inject_early_done, txc)); 
    }

    drop(tx);

    // join producers
    for h in p_handles {
        h.join().unwrap();
    }
    
    dispatcher_handle.join().unwrap();
   
    // join consumers
    for h in c_handles {
        h.join().unwrap();
    }


    println!("all done");
    let final_map = Arc::try_unwrap(shared_map).unwrap().into_inner().unwrap();
    return final_map;
}

fn spawn_producer(id: i32, msgs_per_producer: i32, inject_early_done: bool, tx: mpsc::Sender<Msg>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for n in 0..msgs_per_producer {
            // injecting some funny business
            if inject_early_done && n == 3 {
                tx.send(Msg::Done { producer: id }).unwrap();
            }
            let k = id * 10 + n;
            tx.send(Msg::Data { producer: id, value: k }).unwrap();
            //println!("Sent {k}");
        }
        tx.send(Msg::Done { producer: id }).unwrap();
    }) 
}

pub fn process_msg(msg: Msg, state: &mut HashMap<i32, Data>) -> bool {
    let mut is_shutdown = false;
    match msg {
        Msg::Data { producer, value } => {
            let entry = state.entry(producer).or_default();
            if !entry.closed {
                entry.sum += value;
            }
        }
        Msg::Done { producer } => {
            let entry = state.entry(producer).or_default();
            if !entry.closed {
                entry.closed = true;
            }
        }
        Msg::Shutdown => {
            is_shutdown = true;
        }
    }

    return is_shutdown;
}

pub fn consumer_loop(id: i32, rx: mpsc::Receiver<Msg>, shared_state: Arc<Mutex<HashMap<i32, Data>>>) {
    println!("entering consumer_loop {id}");
    while let Ok(k) = rx.recv() {
        let mut state = shared_state.lock().unwrap();
        if process_msg(k, &mut state) {
            break;
        }
    }
    println!("exiting consumer_loop {id}");
}


#[cfg(test)]
mod tests {
    use super::*;

    fn expected_sum(producer: i32, count: usize) -> i32 {
        (0..count).map(|n| producer * 10 + n as i32).sum()
    }

    fn assert_completes_within<F>(timeout: Duration, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            f();
            let _ = tx.send(());
        });

        match rx.recv_timeout(timeout) {
            Ok(_) => {} // success
            Err(mpsc::RecvTimeoutError::Timeout) => {
                panic!("operation timed out after {:?}", timeout);
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn normal_execution_all_data_counted() {
        let result = run_system(
            3,     // producers
            2,     // consumers
            5,     // messages per producer
            false, // no early Done
        );

        for p in 0..3 {
            let data = result.get(&p).expect("missing producer");
            assert!(data.closed, "producer {p} not closed");
            assert_eq!(
                data.sum,
                expected_sum(p, 5),
                "incorrect sum for producer {p}"
            );
        }
    }

    #[test]
    fn no_double_done_corruption() {
        let result = run_system(3, 2, 5, true);

        for (_, data) in result {
            assert!(data.sum >= 0);
        }
    }

    // -------------------------

    #[test]
    fn single_consumer_all_data_counted() {
        let result = run_system(3, 1, 5, false);

        for p in 0..3 {
            let data = result.get(&p).unwrap();
            assert!(data.closed);
            assert_eq!(data.sum, expected_sum(p, 5));
        }
    }

    #[test]
    fn multiple_consumers_same_result() {
        let result = run_system(3, 4, 5, false);

        for p in 0..3 {
            let data = result.get(&p).unwrap();
            assert!(data.closed);
            assert_eq!(data.sum, expected_sum(p, 5));
        }
    }

    #[test]
    fn early_done_ignores_late_data() {
        let result = run_system(3, 3, 5, true);

        for p in 0..3 {
            let data = result.get(&p).unwrap();
            assert!(data.closed);
            assert_eq!(data.sum, expected_sum(p, 3));
        }
    }

    #[test]
    fn no_double_counting_with_many_consumers() {
        let result = run_system(5, 8, 100, false);

        for p in 0..5 {
            let data = result.get(&p).unwrap();
            assert_eq!(data.sum, expected_sum(p, 100));
        }
    }

    #[test]
    fn system_terminates_cleanly() {
        assert_completes_within(
            Duration::from_secs(2), 
            || {
                let _ = run_system(5, 5, 10, false);
            },
        );
    }
}
