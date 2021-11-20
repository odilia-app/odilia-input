#[macro_use]
extern crate lazy_static;

use std::any::type_name;
use std::future::{Future};
use std::collections::{BTreeSet};
use std::sync::{Arc,Mutex};
use rdev::{
  grab_async,
  Event,
  EventType,
  EventType::{KeyPress, KeyRelease},
  Key
};


fn type_of<T>(_: T) -> &'static str {
      type_name::<T>()
}

#[derive(Copy, Clone)]
pub struct Modifiers {
  pub caps_lock: bool,
  pub l_shift: bool,
  pub l_alt: bool,
  pub l_ctrl: bool,
  pub l_meta: bool,
  pub r_shift: bool,
  pub r_alt: bool,
  pub r_ctrl: bool,
  pub r_meta: bool,
}

impl Modifiers {
  fn new() -> Modifiers {
    Modifiers {
      caps_lock: false,
      l_shift: false,
      l_alt: false,
      l_ctrl: false,
      l_meta: false,
      r_shift: false,
      r_alt: false,
      r_ctrl: false,
      r_meta: false,
    }
  }
}

type AsyncFn = Box<dyn Fn(Event) -> Box<dyn Future<Output = Option<Event>>> + Send>;

lazy_static! {
  static ref CURRENT_MODIFIERS: Arc<Mutex<Modifiers>> = Arc::new(Mutex::new(Modifiers::new()));
}
static TEST: Arc<Mutex<AsyncFn>> = Arc::new(Mutex::new(Box::new(|ev: Event| {
  Box::new(
    Box::new(|| async move{
      Some(ev)
    })()
  )
})));


async fn internal_listener(event: Event) -> Option<Event>{
    let aclone = Arc::clone(&CURRENT_MODIFIERS);
    match event.event_type {
      KeyPress(x) => {
        if let Ok(mut mods) = aclone.lock() {
            match x {
              Key::CapsLock => mods.caps_lock = true,
              Key::ControlLeft => mods.l_ctrl = true,
              Key::ControlRight => mods.r_ctrl = true,
              Key::Alt => mods.l_alt = true,
              Key::AltGr => mods.r_alt = true,
              Key::MetaLeft => mods.l_meta = true,
              Key::MetaRight => mods.r_meta = true,
              Key::ShiftLeft => mods.l_shift = true,
              Key::ShiftRight => mods.r_shift = true,
              _ => {}
            }
          }
          let consume: Box<dyn Future<Output=bool>> = Box::new(async move {
            true
          });
          if consume.unpin() {
            return None;
          } else {
            return Some(event);
          }
      }
      KeyRelease(x) => {
        if let Ok(mut mods) = aclone.lock() {
        match x {
            Key::CapsLock => mods.caps_lock = false,
            Key::ControlLeft => mods.l_ctrl = false,
            Key::ControlRight => mods.r_ctrl = false,
            Key::Alt => mods.l_alt = false,
            Key::AltGr => mods.r_alt = false,
            Key::MetaLeft => mods.l_meta = false,
            Key::MetaRight => mods.r_meta = false,
            Key::ShiftLeft => mods.l_shift = false,
            Key::ShiftRight => mods.r_shift = false,
            _ => {}
          }
        } 
      }
    _ => ()
  }
  Some(event)
}


pub async fn initialize_key_register<F, T>(callback: F) 
where
  F: Fn(Modifiers, Key) -> T + Send + 'static,
  T: Future<Output=bool> + Send + 'static,
{
  if let Ok(tf) = TEST.lock() {
    *tf = callback;
  }
  // this will spawn a new thread, which calls internal listener whenever a key is pressed.
  if let Err(error) = grab_async(internal_listener).await {
    panic!("Error with key handler: {:?}", error);
  }
}
