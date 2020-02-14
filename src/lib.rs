use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::channel::oneshot;
use futures::future::{Fuse, FusedFuture, FutureExt};
use futures::ready;
use futures::task::{Context, Poll};
use pin_project_lite::pin_project;

pub enum StopReason<T> {
    HandleLost,
    Requested(T),
}

impl<T> fmt::Display for StopReason<T>
    where
        T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StopReason::HandleLost => write!(f, "handle lost"),
            StopReason::Requested(r) => write!(f, "requested with reason `{}`", r),
        }
    }
}

impl<T> fmt::Debug for StopReason<T>
    where
        T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StopReason::HandleLost => write!(f, "HandleLost"),
            StopReason::Requested(r) => write!(f, "Requested({:?})", r),
        }
    }
}

impl<T> Clone for StopReason<T>
    where
        T: Clone,
{
    fn clone(&self) -> Self {
        match self {
            StopReason::HandleLost => StopReason::HandleLost,
            StopReason::Requested(r) => StopReason::Requested(r.clone()),
        }
    }
}

pub struct StopHandle<T> {
    inner: Arc<Mutex<Option<oneshot::Sender<T>>>>,
}


impl<T> fmt::Debug for StopHandle<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StopHandle")
    }
}

impl<T> Clone for StopHandle<T> {
    fn clone(&self) -> Self {
        StopHandle {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> StopHandle<T> {
    pub fn stop(&self, reason: T) {
        if let Some(tx) = self.inner.lock().unwrap().take() {
            let _ = tx.send(reason);
        }
    }
}

pin_project! {
    pub struct StopWait<T> {
        #[pin]
        inner: Fuse<oneshot::Receiver<T>>,
    }
}

impl<T> FusedFuture for StopWait<T> {
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}

impl<T> Future for StopWait<T> {
    type Output = StopReason<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let r = match ready!(Future::poll(self.project().inner, cx)) {
            Err(_) => StopReason::HandleLost,
            Ok(reason) => StopReason::Requested(reason),
        };
        Poll::Ready(r)
    }
}

pub fn stop_handle<T>() -> (StopHandle<T>, StopWait<T>) {
    let (tx, rx) = oneshot::channel();
    let stop_handle = StopHandle {
        inner: Arc::new(Mutex::new(Some(tx))),
    };

    let stop_wait = StopWait { inner: rx.fuse() };

    (stop_handle, stop_wait)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use matches::assert_matches;
    use tokio::time::delay_for;

    use super::*;

    #[derive(Debug)]
    pub enum TerminationReason {
        Manual,
    }

    #[tokio::test]
    async fn test_stop_handle() {
        let (stop_handle, stop_wait) = stop_handle();

        tokio::spawn(async move {
            delay_for(Duration::from_secs(1)).await;
            stop_handle.stop(TerminationReason::Manual);
        });

        let res = stop_wait.await;
        assert_matches!(res, StopReason::Requested(TerminationReason::Manual));
    }
}
