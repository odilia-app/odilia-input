use tokio::sync::mpsc;

use once_cell::sync::OnceCell;

const MAX_EVENTS: usize = 256;

thread_local! {
    static TX: OnceCell<mpsc::Sender<rdev::Event>> = OnceCell::new();
}

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
