use tokio::sync::mpsc;

use once_cell::sync::OnceCell;

/// An action to take when an input event arrives
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventAction {
    /// Don't consume the event, allow it to pass through to the application
    Passthrough,
    /// Don't consume the event, but notify us that it happened. Useful for macros, where you want
    /// to record the events, but also want them to pass through
    Notify,
    /// Consume the event, and notify us of it. Used for screen reader commands and key bindings
    Consume,
}

impl EventAction {
    /// Returns `true` if this action will cause the event to be dispatched to us via the
    /// [`mpsc::channel`].
    pub fn notify(self) -> bool {
        matches!(self, Self::Notify | Self::Consume)
    }
}

/// The maximum number of `[rdev::Event`]s that can be in the input queue at one time.
/// The queue could be unbounded, but this allows for backpressure, which allows us to catch up if
/// we get spammed with events.
///
/// On x86_64-unknown-linux-gnu, [`rdev::Event`] is 64 bytes, so this is 16 KiB of queue.
const MAX_EVENTS: usize = 256;

// These are to be used only from the input monitoring thread
thread_local! {
    /// The channel's [`mpsc::Sender`].
    static TX: OnceCell<mpsc::Sender<rdev::Event>> = OnceCell::new();
    /// A function used to decide whether to consume the [`Event`][rdev::Event], and also whether
    /// to notify us of it.
    static DECIDE_ACTION: OnceCell<Box<dyn Fn(&rdev::Event) -> EventAction + Send>> = OnceCell::new();
}

/// Initialise the input monitoring system, returning an [`mpsc::Receiver`] which can be used to
/// recieve input events.
///
/// `decide_action` will be used to determine whether the [`Event`][rdev::Event] is consumed, and
/// also whether we are notified about it via the channel.
/// # Panics
/// * If called more than once in the same program.
pub fn init<F>(decide_action: F) -> mpsc::Receiver<rdev::Event>
where
    F: Fn(&rdev::Event) -> EventAction + Send + 'static,
{
    // Create the channel for communication between the input monitoring thread and async tasks
    let (tx, rx) = mpsc::channel(MAX_EVENTS);

    // Spawn a synchronous input monitoring thread
    std::thread::spawn(move || {
        // Set the thread-local variables
        TX.with(|global| global.set(tx).unwrap());
        DECIDE_ACTION.with(|global| {
            // We can't unwrap() here because the Err variant holds a Box<dyn Fn(...) ...>, which
            // doesn't implement Debug
            if global.set(Box::new(decide_action)).is_err() {
                panic!("init() should only be called once");
            }
        });

        // Start the event loop
        rdev::grab(|ev| {
            TX.with(|tx| {
                let tx = tx.get().unwrap();

                // Decide what to do with this `Event`
                let action = DECIDE_ACTION.with(|decide_action| decide_action.get().unwrap()(&ev));

                if action.notify() {
                    // Notify us by sending the `Event` down the channel
                    if let Err(e) = tx.blocking_send(ev.clone()) {
                        eprintln!("Warning: Failed to process key event: {}", e);
                    }
                }
                // Decide whether to consume the action or pass it through
                if action == EventAction::Consume {
                    None
                } else {
                    Some(ev)
                }
            })
        })
    });

    rx // Return the receiving end of the channel
}
