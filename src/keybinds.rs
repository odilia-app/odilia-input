use odilia_common::{
  input::{
    KeyBinding,
    KeyEvent,
    Modifiers,
  },
  modes::{
    ScreenReaderMode,
  },
};
use tokio::{
  sync::Mutex,
  task::spawn,
};
use std::{
  future::Future,
  collections::HashMap,
};

lazy_static! {
  static ref KB_MAP: Mutex<HashMap<KeyBinding, AsyncFn>> = Mutex::new(HashMap::new());
  static ref SR_MODE: Mutex<ScreenReaderMode> = Mutex::new(ScreenReaderMode::new("CommandMoode"));
}

pub type AsyncFn = Box<dyn Fn() -> Box<dyn Future<Output = ()> + Unpin + Send + 'static> + Send + Sync + 'static>;

async fn boxit<T, F>(func: T) -> AsyncFn 
where
  T: Fn() -> F + Send + Sync + 'static,
  F: Future<Output=()> + Send + 'static
{
  /* if we want to accept arguments, pass them through this syncronous closure */
  Box::new(move || {
    Box::new(Box::pin(
      func()
    ))
  })
}

pub async fn add_keybind<T, F>(kb: KeyBinding, func: T) -> bool 
where
  T: Fn() -> F + Send + Sync + 'static,
  F: Future<Output=()> + Send + 'static
{
  /* WTF? Why can't I check if it didn't workk? I guess tokio mutexes are better somehow? */
  let mut kbhm = KB_MAP.lock().await;
  kbhm.insert(kb, boxit(func).await);
  true
}

pub async fn remove_keybind(kb: KeyBinding) -> bool {
  let mut kbhm = KB_MAP.lock().await;
  kbhm.remove(&kb);
  true
}

pub async fn keyevent_match(kbm: &KeyEvent) -> Option<KeyBinding>
{
  let kbhm = KB_MAP.lock().await;
  let sr_mode = get_sr_mode().await;
  for (kb, _) in kbhm.iter() {
    let mut matches = true;
    matches &= kb.key == kbm.key;
    matches &= kb.repeat == kbm.repeat;
    matches &= (kb.mods == Modifiers::NONE && kbm.mods == Modifiers::NONE) || kb.mods.intersects(kbm.mods);
    if let Some(mode) = &kb.mode {
      matches &= *mode == sr_mode;
    }
    if matches {
      return Some(kb.clone());
    }
  }
  None
}

/* this will match with the bitflags */
pub fn keyevent_match_sync(kbm: &KeyEvent) -> Option<KeyBinding>
{
  let kbhm = KB_MAP.blocking_lock();
  let sr_mode = get_sr_mode_sync();
  for (kb, _) in kbhm.iter() {
    let mut matches = true;
    matches &= kb.key == kbm.key;
    matches &= kb.repeat == kbm.repeat;
    matches &= (kb.mods == Modifiers::NONE && kbm.mods == Modifiers::NONE) || kb.mods.intersects(kbm.mods);
    if let Some(mode) = &kb.mode {
      matches &= *mode == sr_mode;
    }
    if matches {
      return Some(kb.clone());
    }
  }
  None
} 

pub fn get_sr_mode_sync() -> ScreenReaderMode {
  SR_MODE.blocking_lock().clone()
}
pub fn set_sr_mode_sync(srm: ScreenReaderMode) { 
  let mut sr_mode = SR_MODE.blocking_lock();
  *sr_mode = srm;
}
pub async fn get_sr_mode() -> ScreenReaderMode {
  SR_MODE.lock().await.clone()
}
pub async fn set_sr_mode(srm: ScreenReaderMode) {
  let mut sr_mode = SR_MODE.lock().await;
  *sr_mode = srm;
}

/* this is to bridge with events.rs; now init_keyhandlers will be all handled within odilia-input */
pub fn decide_action(ke: &KeyEvent) -> (bool, bool) {
  if let Some(kb) = keyevent_match_sync(ke) {
    return (kb.notify, kb.consume);
  }
  return (false, false);
}

/* TODO: do sync version */
pub async fn run_keybind_func(kb: &KeyBinding) {
  let kbhm = KB_MAP.lock().await;
  let func = kbhm.get(kb).expect("Key binding not found!");
  spawn(async move {
    func().await;
  });
}
