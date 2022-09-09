use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};

use tokio::sync::oneshot;

include!("futures.rs");

pub fn main() {}

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(4))]
fn nondeterministic_schedule() {
    let x = Arc::new(AtomicI64::new(0)); // Surprisingly, Arc verified faster than Rc
    let x2 = x.clone();
    spawnable_block_on(
        async move {
            let x3 = x2.clone();
            spawn(async move {
                x3.fetch_add(1, Ordering::Relaxed);
            });
            yield_now();
            x2.fetch_add(1, Ordering::Relaxed);
        },
        NondetSchedulingPlan::default(),
    );
    assert_eq!(x.load(Ordering::Relaxed), 2);
}

#[kani::proof(schedule = RoundRobin::default())]
#[kani::unwind(4)]
async fn channel_spawn_det() {
    let (sender, receiver) = oneshot::channel::<u8>();
    spawn(async move {
        sender.send(42).unwrap();
    });
    let received = receiver.await;
    assert_eq!(received, Ok(42));
}

#[kani::proof(schedule = RoundRobin::default())]
#[kani::unwind(4)]
async fn channel_spawn_nondet() {
    let (sender, receiver) = oneshot::channel::<u8>();
    let x: u8 = kani::any();
    spawn(async move {
        sender.send(x).unwrap();
    });
    let received = receiver.await;
    assert_eq!(received, Ok(x));
}

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(2))]
async fn channel_nondet() {
    let x: u8 = kani::any();
    let (sender, receiver) = oneshot::channel::<u8>();
    sender.send(x).unwrap();
    let received = receiver.await;
    assert_eq!(received, Ok(x));
}

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(2))]
async fn channel_det() {
    let (sender, receiver) = oneshot::channel::<u8>();
    sender.send(42).unwrap();
    let received = receiver.await;
    assert_eq!(received, Ok(42));
}
