use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

pub enum WatchEvent {
    Changed(std::path::PathBuf),
}

pub fn spawn_watcher(root: &Path) -> anyhow::Result<Receiver<WatchEvent>> {
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                        for path in event.paths {
                            let _ = tx.send(WatchEvent::Changed(path));
                        }
                    }
                    _ => {}
                }
            }
        },
        Config::default().with_poll_interval(Duration::from_millis(200)),
    )?;

    watcher.watch(root, RecursiveMode::Recursive)?;
    // Keep watcher alive for process lifetime
    std::mem::forget(watcher);
    Ok(rx)
}
