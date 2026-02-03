use std::{
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use sitrep::{Controller, Event, Progress, State, StdMpscObserver, Task, UpdateEvent};

fn main() {
    let (sender, receiver) = mpsc::channel();
    let observer = Arc::new(StdMpscObserver::from(sender));

    let (progress, controller) = Progress::new(Task::default(), observer);

    // The sending end of the progress report:
    let worker_handle = thread::spawn(move || {
        let total = 100;
        progress.set_total(total);

        for completed in 1..=total {
            thread::sleep(Duration::from_millis(25));

            // Check if the user canceled the task from the controller end of things:
            if progress.state() == State::Canceled {
                println!("Task got canceled by user, aborting.");
                break;
            }

            progress.set_completed(completed);
        }
    });

    // The receiving end of the progress report:
    let controller_handle = thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            let Event::Update(UpdateEvent { id: _ }) = event else {
                // For the sake of brevity we'll only handle the update events here:
                continue;
            };

            // The controller is only available as long as
            // the corresponding progress is alive, too:
            let Some(controller) = controller.upgrade() else {
                break;
            };

            // Cancel the task from the controller end of things:
            controller.cancel().ok();
        }
    });

    worker_handle.join().unwrap();
    controller_handle.join().unwrap();
}
