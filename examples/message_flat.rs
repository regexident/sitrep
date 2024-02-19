use std::{
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use sitrep::{Event, MessageEvent, Progress, StdMpscObserver, Task};

fn main() {
    let (sender, receiver) = mpsc::channel();
    let observer = Arc::new(StdMpscObserver::from(sender));

    let (progress, _reporter) = Progress::new(Task::default(), observer);

    // The sending end of the progress report:
    let worker_handle = thread::spawn(move || {
        progress.set_label(Some("Crunching numbers ...".into()));

        let total = 100;
        progress.set_total(total);

        for completed in 1..=total {
            thread::sleep(Duration::from_millis(25));

            if completed % 25 == 0 {
                progress.info(|| "Reached a multiple of 25!");
            }

            progress.set_completed(completed);
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
