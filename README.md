# `sitrep`

[![Crates.io](https://img.shields.io/crates/v/sitrep)](https://crates.io/crates/sitrep)
[![Crates.io](https://img.shields.io/crates/d/sitrep)](https://crates.io/crates/sitrep)
[![Crates.io](https://img.shields.io/crates/l/sitrep)](https://crates.io/crates/sitrep)
[![docs.rs](https://docs.rs/sitrep/badge.svg)](https://docs.rs/sitrep/)

Frontend-agnostic progress reporting.

----

## Usage

```rust
use std::{sync::{mpsc, Arc}, thread, time::Duration};

use sitrep::{
    Event, MessageEvent, Progress, DetachmentEvent, Reporter, StdMpscObserver, Task, UpdateEvent,
};

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
            thread::sleep(Duration::from_millis(100));

            progress.set_completed(completed);
        }
    });

    // The receiving end of the progress report:
    let reporter_handle = thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            let Event::Update(UpdateEvent { id }) = event else {
                continue;
            };

            // The reporter is only available as long as
            // the corresponding progress is alive, too:
            let Some(reporter) = reporter.upgrade() else {
                break;
            };

            println!("{:#?}", reporter.report());
        }
    });

    worker_handle.join().unwrap();
    reporter_handle.join().unwrap();
}
```

See the [examples](examples/) directory for more examples.

## Documentation

Please refer to the documentation on [docs.rs](https://docs.rs/sitrep).

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our [code of conduct](https://www.rust-lang.org/conduct.html),  
and the process for submitting pull requests to us.

## Versioning

We use [SemVer](http://semver.org/) for versioning. For the versions available, see the [tags on this repository](https://github.com/regexident/sitrep/tags).

## License

This project is licensed under the [**MPL-2.0**](https://www.tldrlegal.com/l/mpl-2.0) – see the [LICENSE.md](LICENSE.md) file for details.
