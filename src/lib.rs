use tokio::sync::mpsc;

use once_cell::sync::OnceCell;

const MAX_EVENTS: usize = 256;

thread_local! {
    static TX: OnceCell<mpsc::Sender<rdev::Event>> = OnceCell::new();
}

/// Initialise the input monitoring system, returning an [`mpsc::Receiver`] which can be used to
/// recieve input events.
/// # Panics
/// * If called more than once in the same program.
pub fn init() -> mpsc::Receiver<rdev::Event> {
    let (tx, rx) = mpsc::channel(MAX_EVENTS);
    std::thread::spawn(move || {
        TX.with(|global| global.set(tx).unwrap());
        rdev::grab(|ev| {
            TX.with(|tx| {
                let tx = tx.get().unwrap();
                if let Err(e) = tx.blocking_send(ev.clone()) {
                    eprintln!("Warning: Failed to process key event: {}", e);
                }
            });
            Some(ev)
        })
    });
    rx
}
