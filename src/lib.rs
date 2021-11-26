use odilia_common::{
  input::{
    KeyBinding,
    Key,
    Modifiers,
  },
  modes::{
    ScreenReaderMode,
  },
};
use tokio::sync::mpsc;
use rdev::{
  Event,
  EventType::{KeyPress, KeyRelease},
  Key as RDevKey
};

use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::future::Future;

type AsyncFn = Box<dyn Fn() -> Box<dyn Future<Output=()> + Unpin + Send + 'static> + Send + 'static + Sync>;

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

// These are to be used only from the input monitoring thread
thread_local! {
    /// The channel's [`mpsc::Sender`].
    static TX: OnceCell<mpsc::Sender<rdev::Event>> = OnceCell::new();
    /// A function used to decide whether to consume the [`Event`][rdev::Event], and also whether
    /// to notify us of it.
    static DECIDE_ACTION: OnceCell<Box<dyn Fn(&rdev::Event) -> EventAction + Send>> = OnceCell::new();
    static CURRENT_KEYS: Vec<RDevKey> = Vec::new();
    static LAST_KEYS: Vec<RDevKey> = Vec::new();
    static KEY_BINDING_FUNCS: HashMap<KeyBinding, AsyncFn> = HashMap::new();
}

fn vector_eq(va: &Vec<RDevKey>, vb: &Vec<RDevKey>) -> bool {
  (va.len() == vb.len()) &&
  va.iter()
    .zip(vb)
    .all(|(a,b)| format!("{:?}", a) == format!("{:?}", b))
}

fn rdev_keys_to_odilia_modifiers(keys: &Vec<RDevKey>) -> Modifiers {
  let mut modifiers = Modifiers::empty();
  for k in keys {
    modifiers |= match k {
      k if k == RDevKey::CapsLock => Modifiers::ODILIA,
      k if k == RDevKey::Alt => Modifiers::ALT_L,
      k if k == RDevKey::AltGr => Modifiers::ALT_R,
      k if k == RDevKey::ControlLeft => Modifiers::CTRL_L,
      k if k == RDevKey::ControlRight => Modifiers::CTRL_R,
      k if k == RDevKey::ShiftLeft => Modifiers::SHIFT_L,
      k if k == RDevKey::ShiftRight => Modifiers::SHIFT_R,
      k if k == RDevKey::MetaLeft => Modifiers::META_L,
      k if k == RDevKey::MetaRight => Modifiers::META_R,
      _ => 0 as u16,
    }
  }
  modifiers
}

/* NOTE: this breaks if a user pressed a combination with two letters, i.e.: Ctrl+Shift+a+n, or CapsLock+a+s.
This function will always return the first pressed key (a and a in our examples).
*/
fn rdev_keys_to_single_odilia_key(keys: &Vec<RDevKey>) -> Option<Key> {
  for k in keys {
    let m = match k {
      RDevKey::Backspace => Key::Backspace,
      RDevKey::Delete => Key::Delete,
      RDevKey::DownArrow => Key::Down,
      RDevKey::UpArrow => Key::Up,
      RDevKey::LeftArrow => Key::Left,
      RDevKey::RightArrow => Key::Right,
      RDevKey::End => Key::End,
      RDevKey::Escape => Key::Escape,
      RDevKey::F1 => Key::F1,
      RDevKey::F2 => Key::F2,
      RDevKey::F3 => Key::F3,
      RDevKey::F4 => Key::F4,
      RDevKey::F5 => Key::F5,
      RDevKey::F6 => Key::F6,
      RDevKey::F7 => Key::F7,
      RDevKey::F8 => Key::F8,
      RDevKey::F9 => Key::F9,
      RDevKey::F10 => Key::F10,
      RDevKey::F11 => Key::F11,
      RDevKey::F12 => Key::F12,
      RDevKey::Home => Key::Home,
      RDevKey::PageDown => Key::PageDown,
      RDevKey::PageUp => Key::PageUp,
      RDevKey::Return => Key::Return,
      RdevKey::Space => Key::Space,
      RDevKey::Tab => Key::Tab,
      RDevKey::PrintScreen => Key::PrintScreen,
      RDevKey::ScrollLock => Key::ScrollLock,
      RDevKey::Pause => Key::Pause,
      RDevKey::NumLock => Key::NumLock,
      RDevKey::BackQuote => Key::Other('`'),
      RDevKey::Num0 => Key::Kp0,
      RDevKey::Num1 => Key::Kp1,
      RDevKey::Num2 => Key::Kp2,
      RDevKey::Num3 => Key::Kp3,
      RDevKey::Num4 => Key::Kp4,
      RDevKey::Num5 => Key::Kp5,
      RDevKey::Num6 => Key::Kp6,
      RDevKey::Num7 => Key::Kp7,
      RDevKey::Num8 => Key::Kp8,
      RDevKey::Num9 => Key::Kp9,
      RDevKey::Minus => Key::Other('-'),
      RDevKey::Equal => Key::Other('='),
      RDevKey::KeyQ => Key::Other('q'),
      RDevKey::KeyW => Key::Other('w'),
      RDevKey::KeyE => Key::Other('e'),
      RDevKey::KeyR => Key::Other('r'),
      RDevKey::KeyT => Key::Other('t'),
      RDevKey::KeyY => Key::Other('y'),
      RDevKey::KeyU => Key::Other('u'),
      RDevKey::KeyI => Key::Other('i'),
      RDevKey::KeyO => Key::Other('o'),
      RDevKey::KeyP => Key::Other('p'),
      RDevKey::LeftBracket => Key::Other('['),
      RDevKey::RightBracket => Key::Other(']'),
      RDevKey::KeyA => Key::Other('a'),
      RDevKey::KeyS => Key::Other('s'),
      RDevKey::KeyD => Key::Other('d'),
      RDevKey::KeyF => Key::Other('f'),
      RDevKey::KeyG => Key::Other('g'),
      RDevKey::KeyH => Key::Other('h'),
      RDevKey::KeyJ => Key::Other('j'),
      RDevKey::KeyK => Key::Other('k'),
      RDevKey::KeyL => Key::Other('l'),
      RDevKey::SemiColon => Key::Other(';'),
      RDevKey::Quote => Key::Other('\''),
      RDevKey::BackSlack => Key::Other('\\'),
      // TODO: check if correct below
      RDevKey::IntlBackslash => Key::Other('\\'),
      RDevKey::KeyZ => Key::Other('z'),
      RDevKey::KeyX => Key::Other('x'),
      RDevKey::KeyC => Key::Other('c'),
      RDevKey::KeyV => Key::Other('v'),
      RDevKey::KeyB => Key::Other('b'),
      RDevKey::KeyN => Key::Other('n'),
      RDevKey::Comma => Key::Other(','),
      RDevKey::Dot => Key::Other('.'),
      RDevKey::Slash => Key::Other('/'),
      RDevKey::Insert => Key::Insert,
      RDevKey::KpReturn => Key::KpReturn,
      RDevKey::KpMinus => Key::KpMinus,
      RDevKey::KpPlus => Key::KpPlus,
      RDevKey::KpMultiply => Key::KpMultiply,
      RDevKey::KpDivide => Key::KpDivide,
      RDevKey::KpDelete => Key::KpDelete,
      RDevKey::Function => Key::Function,
      _ => None,
    };
    if let Some(m2) = m {
      return m2;
    }
  }
  return None;
}

fn keybind_match(key: Key, mods: Modifiers, repeat: u8, mode: Option<ScreenReaderMode>, consume: Option<bool>, keybindings: &HashMap<KeyBinding, AsyncFn>) -> Option<AsyncFn> {
  for (kb, afn) in keybindings {
    let mut matched = true;
    if kb.key == key {
      matched &= true;
    } else {
      matched &= false;
    }
    if kb.mods = mods {
      matched &= true;
    } else {
      matched &= false;
    }
    if let Some(c) = consume {
      if kb.consume == c {
        matched &= true;
      } else {
        matched &= false;
      }
    } else {
      matched &= true;
    }

    if let Some(m) = mode {
      if kb.mode == m {
        matched &= true;
      } else {
        matched &= false;
      }
    } else {
      matched &= true;
    }

    if matched {
      return afn;
    }
  }
  None
}

/* Option so None can be returned if "KeyPress" continues to fire while one key continues to be held down */
fn rdev_event_to_func_to_call(event: Event, current_keys: &Vec<RDevKey>, last_keys: &Vec<RDevKey>, kbfncs: &HashMap<KeyBinding, AsyncFn>) -> Option<AsyncFn> {
  match event.event_type {
    KeyPress(x) => {
      last_keys = current_keys.clone();
      current_keys.push(x);
      current_keys.dedup();
      // if there is a new key pressed/released and it is not a repeat event
      if !vec_eq(&last_keys, &current_keys) {
        let key = rdev_keys_to_single_odilia_key(&current_keys);
        let mods = rdev_keys_to_odilia_modifiers(&current_keys);
        keybind_match(
          key,
          mods,
          1 as u8, // fixed for now
          None, // match all modes
          None, // match consume and not consume
        )
      }
    },
    KeyRelease(x) => {
      last_keys = current_keys.clone();
      // remove just released key from curent keys
      current_keys.retain(|&k| k != x);
      None
    },
    _ => None
  }
}

/// The maximum number of `[rdev::Event`]s that can be in the input queue at one time.
/// The queue could be unbounded, but this allows for backpressure, which allows us to catch up if
/// we get spammed with events.
///
/// On x86_64-unknown-linux-gnu, [`rdev::Event`] is 64 bytes, so this is 16 KiB of queue.
const MAX_EVENTS: usize = 256;


/// Initialise the input monitoring system, returning an [`mpsc::Receiver`] which can be used to
/// recieve input events.
///
/// `decide_action` will be used to determine whether the [`Event`][rdev::Event] is consumed, and
/// also whether we are notified about it via the channel.
/// # Panics
/// * If called more than once in the same program.
pub fn init<F>(decide_action: F, keymap: &HashMap<KeyBinding, AsyncFn> + Sync) -> mpsc::Receiver<rdev::Event>
where
    F: Fn(&rdev::Event) -> EventAction + Send + 'static,
{

    // Create the channel for communication between the input monitoring thread and async tasks
    let (tx, rx) = mpsc::channel(MAX_EVENTS);

    // Spawn a synchronous input monitoring thread
    std::thread::spawn(move || {
        // Set the thread-local variables
        TX.with(|global| global.set(tx).unwrap());
        CURRENT_KEYS.with(|global| global = &Vec::new() );
        LAST_KEYS.with(|global| global = &Vec::new() );
        KEY_BINDING_FUNCS.with(|global| global = &keymap.clone() );
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
