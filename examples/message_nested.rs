use std::{
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use sitrep::{Event, MessageEvent, Progress, StdMpscObserver, Task};

fn main() {
    let (sender, receiver) = mpsc::channel();
    let observer = Arc::new(StdMpscObserver::from(sender));

    let (parent, _reporter) = Progress::new(Task::default(), observer);

    // The sending end of the progress report:
    let worker_handle = thread::spawn(move || {
        let worker_handles: Vec<_> = (0..3)
            .map(|_| {
                let parent = Arc::clone(&parent);
                thread::spawn(move || {
                    let child = Progress::new_with_parent(Task::default(), &parent);

                    let total = 100;

                    for completed in 1..=total {
                        thread::sleep(Duration::from_millis(25));

                        if completed % 25 == 0 {
                            child.info(|| "Reached a multiple of 25!");
                        }
                    }
                })
            })
            .collect();

        for worker_handle in worker_handles {
            worker_handle.join().unwrap();
        }
    });

    // The receiving end of the progress report:
    let reporter_handle = thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            let Event::Message(MessageEvent {
                id,
                message,
                priority,
            }) = event
            else {
                // For the sake of brevity we'll only handle the message events here:
                continue;
            };

            println!("Progress {id:?} messaged ({priority:?}): {message:?}");
        }
    });

    worker_handle.join().unwrap();
    reporter_handle.join().unwrap();
}
