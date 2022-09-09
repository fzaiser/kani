use async_oneshot;

pub fn main() {}

#[kani::proof(schedule = RoundRobin::default())]
#[kani::unwind(5)]
async fn channel_verify() {
    let x: u8 = kani::any();
    let (mut sender, receiver) = async_oneshot::oneshot::<u8>();
    spawn(async move {
        sender.send(x).unwrap();
    });
    let received = receiver.await;
    assert_eq!(received, Ok(x));
}
