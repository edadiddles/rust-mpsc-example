use std::sync::mpsc;
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel();

    let mut handles = Vec::new();
    // TODO: spawn 3 producer threads
    for i in 0..3 {
        let txc = tx.clone();
        handles.push(thread::spawn(move || {
            for n in 0..5 {
                let k = i*10 + n;
                txc.send(k).unwrap();
                println!("Sent {k}");
            }
        }));
    }

    // TODO: drop the original sender if needed
    drop(tx);

    // Consumer
    for value in rx {
        println!("received {}", value);
    }

    for h in handles {
        h.join().unwrap()
    }

    println!("all done");
}
