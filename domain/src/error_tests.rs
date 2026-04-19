use super::*;
use std::{
    error::Error as StdError,
    sync::{Arc, Mutex},
};

#[test]
fn from_io_error_preserves_message() {
    let io_err = io::Error::other("disk full");
    let err = Error::from(io_err);
    assert!(err.to_string().contains("disk full"));
    assert!(StdError::source(&err).is_some());
}

#[test]
fn from_poison_error_extracts_message() {
    let mutex = Arc::new(Mutex::new(0i32));
    let m2 = Arc::clone(&mutex);
    std::thread::spawn(move || {
        let _guard = m2.lock().unwrap();
        panic!("intentional poison");
    })
    .join()
    .ok();
    let err: Error = mutex.lock().unwrap_err().into();
    assert!(err.to_string().contains("poisoned lock"));
    assert!(StdError::source(&err).is_none());
}
