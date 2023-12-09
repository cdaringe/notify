use futures::{
    channel::mpsc::{
        channel,
        Receiver,
        // SendError
    },
    SinkExt, StreamExt,
};
use notify::{
    // Config,
    // Event,
    // RecommendedWatcher,
    FsEventWatcher,
    RecursiveMode,
    // Watcher,
};

use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};
use std::{
    path::Path,
    // sync::{Arc, RwLock},
    time::Duration,
};

/// Async, futures channel based event watching
fn main() {
    futures::executor::block_on(async { async_watch().await.expect_err("watcher failed") });
}

#[derive(Debug)]
struct FsWatcherBuilder {
    options: FsWatchOptions,
}

// #[derive(Debug, Default)]
// enum EventState {
//     #[default]
//     Sending,
//     Halted(SendError),
// }

// type RWEventState = Arc<RwLock<EventState>>;

struct FsWatcher {
    debouncer: Debouncer<FsEventWatcher>,
    receiver: Receiver<Result<Vec<DebouncedEvent>, notify::Error>>,
    // event_state: RWEventState,
    options: FsWatchOptions,
}

#[derive(Debug, Default)]
pub enum FsWatchRecursiveMode {
    #[default]
    Flat,
    Recursive,
}

#[derive(Debug)]
struct FsWatchOptions {
    debounce_s: u64,
    recursive_mode: FsWatchRecursiveMode,
    path: String,
}

impl Default for FsWatchOptions {
    fn default() -> Self {
        Self {
            debounce_s: Default::default(),
            recursive_mode: Default::default(),
            path: ".".to_string(),
        }
    }
}

#[derive(Debug)]
enum FsWatchError {
    InitError(String),
}

impl FsWatcherBuilder {
    pub fn new() -> Self {
        Self {
            options: FsWatchOptions::default(),
        }
    }

    pub fn debounce(mut self, seconds: u64) -> Self {
        self.options.debounce_s = seconds;
        self
    }

    pub fn recursive(mut self, is_recursive: bool) -> Self {
        self.options.recursive_mode = match is_recursive {
            true => FsWatchRecursiveMode::Recursive,
            _ => FsWatchRecursiveMode::Flat,
        };
        self
    }

    pub fn path(mut self, path: String) -> Self {
        self.options.path = path;
        self
    }

    pub fn build(self) -> Result<FsWatcher, FsWatchError> {
        let (mut tx, receiver) = channel(1);
        // let event_state: RWEventState = Arc::new(RwLock::new(EventState::default()));
        // let local_event_state = event_state.clone();
        let debouncer: Debouncer<FsEventWatcher> =
            new_debouncer(Duration::from_secs(self.options.debounce_s), move |res| {
                futures::executor::block_on(async {
                    match tx.send(res).await {
                        Ok(_) => (),
                        Err(err) => {
                            let msg = err.to_string();
                            // if let Ok(mut current_state) = local_event_state.clone().write() {
                            //     *current_state = EventState::Halted(err);
                            // }
                            // let _ = tx.close().await;
                            panic!("fs watcher local message channel failed: {}", &msg);
                        }
                    }
                })
            })
            .map_err(|e| FsWatchError::InitError(e.to_string()))?;

        Ok(FsWatcher {
            options: self.options,
            debouncer,
            receiver,
            // event_state,
        })
    }
}

impl FsWatcher {
    pub fn watch<'a>(
        &mut self,
    ) -> Result<&mut Receiver<Result<Vec<DebouncedEvent>, notify::Error>>, FsWatchError> {
        let watcher = self.debouncer.watcher();
        let recursive_mode = match self.options.recursive_mode {
            FsWatchRecursiveMode::Flat => RecursiveMode::NonRecursive,
            FsWatchRecursiveMode::Recursive => RecursiveMode::Recursive,
        };
        watcher
            .watch(&Path::new(&self.options.path), recursive_mode)
            .map_err(|e| FsWatchError::InitError(e.to_string()))?;

        Ok(&mut self.receiver)
    }
}

async fn async_watch() -> Result<(), FsWatchError> {
    let mut watcher = FsWatcherBuilder::new()
        .debounce(1)
        .recursive(false)
        .path(".".to_owned())
        .build()?;

    loop {
        match watcher.watch()?.next().await {
            Some(res) => match res {
                Ok(event) => println!("changed: {:?}", event),
                Err(e) => println!("watch error: {:?}", e),
            },
            None => return Ok(()),
        }
    }
}
