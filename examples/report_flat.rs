use std::{
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use sitrep::{Event, Progress, Reporter, StdMpscObserver, Task, UpdateEvent};

fn main() {
    let (sender, receiver) = mpsc::channel();
    let observer = Arc::new(StdMpscObserver::from(sender));

    let (progress, reporter) = Progress::new(Task::default(), observer);

    // The sending end of the progress report:
    let worker_handle = thread::spawn(move || {
        progress.set_label(Some("Crunching numbers ...".into()));

        let total = 100;
        progress.set_total(total);

        for completed in 1..=total {
            thread::sleep(Duration::from_millis(25));

            progress.set_completed(completed);
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
