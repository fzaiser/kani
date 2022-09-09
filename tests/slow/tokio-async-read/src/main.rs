
use tokio::io::{AsyncReadExt, AsyncRead};

pub fn main() {}

#[tokio::test]
async fn test() {
    let buffer: [u8; 6] = [2, 4, 6, 8, 10, 12];
    let checksum = compute_checksum(&buffer as &[u8]).await;
    assert_eq!(checksum, 42);
}

async fn compute_checksum<R: AsyncRead + Unpin>(mut input: R) -> u8 {
    let mut checksum = 0u8;
    while let Ok(byte) = input.read_u8().await {
        checksum = checksum.wrapping_add(byte);
    }
    checksum
}

#[cfg(kani)]
#[kani::proof]
#[kani::unwind(4)]
async fn proof() {
    let buffer: [u8; 2] = kani::any();
    let checksum = compute_checksum(&buffer as &[u8]).await;
    assert_eq!(checksum, buffer.iter().fold(0, |a,b| a.wrapping_add(*b)));
}