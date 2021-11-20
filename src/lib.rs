#[macro_use]
extern crate lazy_static;

use tokio::task;
use tokio::sync::Mutex;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::mem::drop;
use std::any::type_name;
use std::future::{Future};
use std::collections::{BTreeSet};
use std::sync::{Arc};
use rdev::{
  grab_async,
  Event,
  EventType,
  EventType::{KeyPress, KeyRelease},
  Key
};

fn vector_equals(va: &Vec<Key>, vb: &Vec<Key>) -> bool{
  (va.len() == vb.len()) &&  // zip stops at the shortest
    va.iter()
    .zip(vb)
    // TODO: very slow
    .all(|(a,b)| format!("{:?}", a) == format!("{:?}", b))
}

fn type_of<T>(_: T) -> &'static str {
      type_name::<T>()
}

type Part = Box<dyn Future<Output=Option<bool>>>;
type AsyncFn = Box<dyn Fn(Vec<Key>) -> Box<dyn Future<Output = bool> + Unpin + Send + 'static> + Send + 'static>;

lazy_static! {
  static ref LAST_KEYS: tokio::sync::Mutex<Vec<Key>> = Mutex::const_new(Vec::new());
  static ref CURRENT_KEYS: tokio::sync::Mutex<Vec<Key>> = Mutex::const_new(Vec::new());
   static ref TEST: tokio::sync::Mutex<AsyncFn> = tokio::sync::Mutex::const_new(
Box::new(move |keys: Vec<Key>| {
  Box::new(Box::pin(async move {
    false
  }))
}));
}


async fn internal_listener(event: Event) -> Option<Event> {
    match event.event_type {
      KeyPress(x) => {
        let mut mods = CURRENT_KEYS.lock().await;
          let mut last_keys = LAST_KEYS.lock().await;
          *last_keys = mods.clone();
         mods.push(x);
            let sf = TEST.lock().await;
                mods.dedup();
                //println!("CURR[D]: {:?}", mods);
                //println!("LAST[D]: {:?}", last_keys);
                if !vector_equals(&mods, &last_keys) {
                  let consume: task::JoinHandle<bool> = task::spawn(sf(mods.clone()));
                  //println!("TYPE: {:?}", type_of(consume.await.unwrap()));
                  if consume.await.unwrap() {
                    return None;
                  } else {
                    return Some(event);
                  }
                } 
            
          
      }
      KeyRelease(x) => {
        let mut last_keys = LAST_KEYS.lock().await;
        let mut mods = CURRENT_KEYS.lock().await;
        *last_keys = mods.clone();
        mods.retain(|&k| k != x);
                //println!("CURR[U]: {:?}", mods);
                //println!("LAST[U]: {:?}", last_keys);
      }
    _ => ()
  }
  Some(event)
}


pub async fn initialize_key_register<F, T>(callback: F) 
where
  F: Fn(Vec<Key>) -> T + Send + 'static,
  T: Future<Output=bool> + Send  + 'static,
{
  let mut tf = TEST.lock().await;
  *tf = Box::new(move |keys: Vec<Key>| {
      Box::new(Box::pin(callback(keys)))
    });
  // this will spawn a new thread, which calls internal listener whenever a key is pressed.
  if let Err(error) = grab_async(internal_listener).await {
    panic!("Error with key handler: {:?}", error);
  }
}
