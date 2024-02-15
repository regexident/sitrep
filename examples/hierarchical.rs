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
        parent.set_label("Crunching numbers ...".to_owned());

        for i in 0..3 {
            let child = Progress::new_with_parent(Task::default(), &parent);

            let total = 100;
            child.set_total(total);

            child.set_label(format!("{}..{}", i * total, i * total + total));

            for completed in 1..=total {
                thread::sleep(Duration::from_millis(100));

                if completed % 25 == 0 {
                    child.info(|| "Reached a multiple of 25!");
                }

                child.set_completed(completed);
            }
        }
    });

    // The receiving end of the progress report:
    let reporter_handle = thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            match event {
                Event::Update(UpdateEvent { id }) => {
                    println!("Progress {id:?} has reported an update");

                    // The reporter is only available as long as
                    // the corresponding progress is alive, too:
                    let Some(reporter) = reporter.upgrade() else {
                        break;
                    };

                    let report = reporter.report();
                    println!("{report:#?}");
                }
                Event::Message(MessageEvent {
                    id,
                    message,
                    priority,
                }) => {
                    println!("A message was posted by progress {id:?} with priority {priority:?}: {message:?}");
                }
                Event::Removed(RemovalEvent { id }) => {
                    println!("Sub-progress {id:?} was removed");
                }
                Event::GenerationOverflow => {
                    // If you're using the report's generation
                    // for change detection, then you need to handle this.
                }
            }
        }
    });

    worker_handle.join().unwrap();
    reporter_handle.join().unwrap();
}
