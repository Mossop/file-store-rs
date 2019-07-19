//! A default executor to use when running futures from this crate.
//!
//! Ideally everything in this crate would run under any executor. In practice
//! some parts have dependencies on tokio's executor. Hopefully that will change
//! in the future though so for now this exposes a way to run futures that will
//! work for this crate.

extern crate tokio;

use std::future::Future;
use std::sync::mpsc;
use std::boxed::Box;

use futures::compat::Compat;
use futures::future::FutureExt;
use futures::channel::oneshot;

/// Runs a future on the existing runtime.
pub fn spawn<F>(future: F) -> impl Future<Output = Result<F::Output, oneshot::Canceled>>
where
    F: Future + Send + 'static,
    F::Output: Send,
{
    let (sender, receiver) = oneshot::channel::<F::Output>();

    let compat = Compat::new(Box::pin(future).map(move |r| match sender.send(r) {
        Ok(()) => Ok(()),
        Err(_) => Err(()),
    }));

    tokio::executor::spawn(compat);

    receiver
}

/// Runs a future to completion on a new tokio executor and returns the result.
pub fn run<F>(future: F) -> Result<F::Output, mpsc::TryRecvError>
where
    F: Future + Send + 'static,
    F::Output: Send,
{
    let (sender, receiver) = mpsc::channel::<F::Output>();

    let compat = Compat::new(Box::pin(future).map(move |r| match sender.send(r) {
        Ok(()) => Ok(()),
        Err(_) => Err(()),
    }));

    tokio::run(compat);

    receiver.try_recv()
}
