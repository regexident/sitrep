use std::{
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use sitrep::{
    Event, MessageEvent, Progress, RemovalEvent, Reporter, StdMpscObserver, Task, UpdateEvent,
};

fn main() {
    let (sender, receiver) = mpsc::channel();
    let observer = Arc::new(StdMpscObserver::from(sender));

    let (parent, reporter) = Progress::new(Task::default(), observer);

    // The sending end of the progress report:
    let worker_handle = thread::spawn(move || {
        parent.set_label(Some("Crunching numbers ...".into()));

        for i in 0..3 {
            let child = Progress::new_with_parent(Task::default(), &parent);

            let total = 100;
            child.set_total(total);

            child.set_label(Some(format!("{}..{}", i * total, i * total + total).into()));

            for completed in 1..=total {
                thread::sleep(Duration::from_millis(25));

                child.set_completed(completed);
            }
        }
    });

    // The receiving end of the progress report:
    let reporter_handle = thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            let Event::Update(UpdateEvent { id: _ }) = event else {
                // For the sake of brevity we'll only handle the update events here:
                continue;
            };

            // The reporter is only available as long as
            // the corresponding progress is alive, too:
            let Some(reporter) = reporter.upgrade() else {
                break;
            };

            let report = reporter.report();

            println!(
                "Progress updated: {fraction}% {label}",
                fraction = 100.0 * report.fraction,
                label = report.label.map_or(String::new(), |cow| (*cow).to_owned())
            );
        }
    });

    worker_handle.join().unwrap();
    reporter_handle.join().unwrap();
}
