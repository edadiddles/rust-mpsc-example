use std::sync::mpsc;
use std::thread;
use std::collections::hash_map::HashMap;

enum Msg {
    Data(i32, i32),
    Done(i32),
}

fn main() {
    let (tx, rx) = mpsc::channel();

    let mut handles = Vec::new();
    // TODO: spawn 3 producer threads
    for i in 0..3 {
        let txc = tx.clone();
        handles.push(thread::spawn(move || {
            for n in 0..5 {
                let k = i*10 + n;
                txc.send(Msg::Data(i, k)).unwrap();
                println!("Sent {k}");
            }
            txc.send(Msg::Done(i)).unwrap();
        }));
    }

    // TODO: drop the original sender if needed
    drop(tx);

    // Consumer
    let mut map: HashMap<i32, i32> = HashMap::new();
    for value in rx {
        match value {
            Msg::Data(x, y) => {
                println!("received {y} from producer {x}");
                let k = map.get(&x).unwrap_or(&0);
                map.insert(x, y+k);
            },
            Msg::Done(x) => {
                println!("producer {x} finished");
                println!("producer {x} sent sum {}", map.get(&x).unwrap_or(&0));
            },
        }
        
    }

    for h in handles {
        h.join().unwrap()
    }

    println!("all done");
}
