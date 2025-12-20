use std::sync::mpsc;
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel();

    // TODO: spawn 3 producer threads
    for i in 0..3 {
        let txc = tx.clone();
        thread::spawn(move || {
            for n in 0..5 {
                let k = i*10 + n;
                txc.send(k).unwrap();
                println!("Sent {k}");
            }
        });
    }

    // TODO: drop the original sender if needed
    drop(tx);

    // Consumer
    for value in rx {
        println!("received {}", value);
    }

    println!("all done");
}
