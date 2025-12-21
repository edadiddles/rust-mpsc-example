use std::sync::mpsc;
use std::thread;
use std::collections::hash_map::HashMap;

enum Msg {
    Data(i32, i32),
    Done(i32),
}

struct Data {
    data_sum: i32,
    is_closed: bool,
}

fn main() {
    let (tx, rx) = mpsc::channel();

    let mut data_map: HashMap<i32, Data> = HashMap::new();

    let mut handles = Vec::new();
    for i in 0..3 {
        let txc = tx.clone();
        data_map.insert(i, Data{
            data_sum: 0,
            is_closed: false,
        });
        handles.push(thread::spawn(move || {
            for n in 0..5 {
                // injecting some funny business
                if i == 1 && n == 2 {
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
                let prod_data = data_map.get_mut(&x);
                match prod_data {
                    Some(m) => {
                        if m.is_closed {
                            println!("producer {x} alread closed");
                            continue;
                        }
                        m.data_sum += y;
                    },
                    None => println!("producer {x} not found"),
                }
            },
            Msg::Done(x) => {
                println!("producer {x} finished");
                let prod_data = data_map.get_mut(&x);
                match prod_data {
                    Some(m) => {
                        println!("producer {x} sent sum {}", m.data_sum);
                        m.is_closed = true;
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
}
