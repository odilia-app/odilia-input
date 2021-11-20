#[macro_use]
extern crate lazy_static;

use std::any::type_name;
use std::pin::Pin;
use std::error::Error;
use std::future::{Future};
use futures::future::BoxFuture;
use std::boxed::Box;
use std::collections::{HashMap};
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

lazy_static! {
  static ref CURRENT_KEYS: Mutex<Vec<Key>> = Mutex::new(Vec::new());
  static ref LISTENERS: Mutex<HashMap<String, Fn(Event) -> bool + 'static>> = Mutex::new(HashMap::new());
  static ref TEST: Mutex<Vec<_>> = Mutex::new(Vec::new());
}

fn key_vec_to_string(keys: &Vec<Key>) -> String {
  // TODO: overhead high, ifnd different way
  keys.iter().map(|k| format!("{:?}", k)).collect::<String>()
}

async fn internal_listener(event: Event) -> Option<Event> {
  match event.event_type {
    KeyPress(x) => {
      if let Ok(mut ck) = CURRENT_KEYS.lock() {
        ck.push(x);
      }
    }
    KeyRelease(x) => {
      if let Ok(mut ck) = CURRENT_KEYS.lock() {
        ck.retain(|&k| k != x);
      }
    }
    _ => ()
  }

  return Some(event)
}

pub async fn initialize_key_register() {
  // this will spawn a new thread, which calls internal listener whenever a key is pressed.
  if let Err(error) = grab_async(internal_listener).await {
    panic!("Error with key handler: {:?}", error);
  }
}

/*
Adds an async callback which returns a boolean of whether to continue propagating everything up the stack, or let it die quickly.

Returns true upon succes, false otherwise.
*/
pub async fn add_listener<T, F>(keys: Vec<Key>, callback: F) -> bool
where
  F: Fn(Event) -> T + Send + 'static,
  T: Future<Output=bool> + Send + 'static,
{
  // how would I save this globally?
  let mut x: Vec<F> = Vec::new();
  x.push(callback);
  println!("LEN: {}", x.len());
  if let Ok(mut listeners) = LISTENERS.lock() {
    listeners.insert(key_vec_to_string(&keys), String::from("hi"));
    println!("TYPE: {:?}", type_of(x.get(0).unwrap()));
    return true
  }
  false
}

/*
Removes listener made with a vector of keys.

Returns true if successful. Otherwise false.
*/
pub async fn remove_listener(keys: Vec<Key>) -> bool
{
  if let Ok(mut listeners) = LISTENERS.lock() {
    listeners.remove(&key_vec_to_string(&keys));
    return true
  }
  false
}
