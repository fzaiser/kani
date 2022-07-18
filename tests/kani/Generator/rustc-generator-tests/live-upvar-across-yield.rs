// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Copyright rustc Contributors
// Adapted from rustc: src/test/ui/generator/live-upvar-across-yield.rs

// run-pass

#![feature(generators, generator_trait)]

use std::ops::Generator;
use std::pin::Pin;

#[kani::proof]
#[kani::unwind(2)]
fn main() {
    let b = |_| 3;
    let mut a = || {
        b(yield);
    };
    Pin::new(&mut a).resume(());
}
