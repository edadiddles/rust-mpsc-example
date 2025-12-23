use std::sync::mpsc;
use std::thread;
use std::collections::hash_map::HashMap;
use std::sync::{Arc, Mutex};

enum Msg {
    Data(i32, i32),
    Done(i32),
}

#[derive(Debug)]
struct Data {
    sum: i32,
    closed: bool,
}

fn main() {
    let _ = run_system(3, 1, 5, false);
}

fn run_system(producers: i32, _consumers: i32, msgs_per_producer: i32, inject_early_done: bool,) -> HashMap<i32, Data> {
    let (tx, rx) = mpsc::channel();
    let shared_map: Arc<Mutex<HashMap<i32, Data>>> = Arc::new(Mutex::new(HashMap::new()));

    let mut handles = Vec::new();
    for i in 0..producers {
        let mut map = shared_map.lock().unwrap();
        map.insert(i, Data{
            sum: 0,
            closed: false,
        });
    }

    for i in 0..producers {
        let txc = tx.clone();
        handles.push(thread::spawn(move || {
            for n in 0..msgs_per_producer {
                // injecting some funny business
                if inject_early_done && n == 3 {
                    txc.send(Msg::Done(i)).unwrap();
                }
                let k = i*10 + n;
                txc.send(Msg::Data(i, k)).unwrap();
                println!("Sent {k}");
            }
            txc.send(Msg::Done(i)).unwrap();
        }));
    }

    drop(tx);

    // Consumer
    for value in rx {
        match value {
            Msg::Data(x, y) => {
                println!("received {y} from producer {x}");
                let mut data_map = shared_map.lock().unwrap();
                let prod_data = data_map.get_mut(&x);
                match prod_data {
                    Some(m) => {
                        if m.closed {
                            println!("producer {x} alread closed");
                            continue;
                        }
                        m.sum += y;
                    },
                    None => println!("producer {x} not found"),
                }
            },
            Msg::Done(x) => {
                println!("producer {x} finished");
                let mut data_map = shared_map.lock().unwrap();
                let prod_data = data_map.get_mut(&x);
                match prod_data {
                    Some(m) => {
                        if m.closed {
                            println!("producer {x} already closed");
                            continue;
                        }
                        println!("producer {x} sent sum {}", m.sum);
                        m.closed = true;
                    },
                    None => println!("producer {x} not found"),
                }
            },
        }
        
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("all done");
    let final_map = Arc::try_unwrap(shared_map).unwrap().into_inner().unwrap();
    return final_map;
}


#[cfg(test)]
mod tests {
    use super::*;

    fn expected_sum(producer: i32, count: usize) -> i32 {
        (0..count)
            .map(|n| producer * 10 + n as i32)
            .sum()
    }

    #[test]
    fn normal_execution_all_data_counted() {
        let result = run_system(
            3,   // producers
            2,   // consumers
            5,   // messages per producer
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
    fn early_done_ignores_late_data() {
        let result = run_system(
            3,
            2,
            5,
            true, // inject Done early
        );

        for p in 0..3 {
            let data = result.get(&p).expect("missing producer");
            assert!(data.closed, "producer {p} not closed");

            // Only values before Done (0,1,2)
            let expected = expected_sum(p, 3);
            assert_eq!(
                data.sum,
                expected,
                "data after Done was incorrectly counted for producer {p}"
            );
        }
    }

    #[test]
    fn no_double_done_corruption() {
        let result = run_system(
            3,
            2,
            5,
            true,
        );

        for (_, data) in result {
            assert!(data.sum >= 0);
        }
    }

    #[test]
    fn system_terminates_cleanly() {
        // If this test hangs, you have a deadlock
        let _ = run_system(3, 2, 5, false);
    }
}
