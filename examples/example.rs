use std::time::Duration;

use tokio::time::delay_for;

use stop_handle::stop_handle;

#[derive(Debug)]
pub enum TerminationReason {
    Manual,
}

#[tokio::main]
pub async fn main() {
    let (stop_handle, stop_wait) = stop_handle();

    tokio::spawn(async move {
        delay_for(Duration::from_secs(1)).await;
        stop_handle.stop(TerminationReason::Manual);
    });

    let res = stop_wait.await;
    println!("terminated with status: {:?}", res);
}
