// Copyright rustc Contributors
// SPDX-License-Identifier: Apache OR MIT
// Adapted from rustc: src/test/ui/generator/control-flow.rs
// Changes: copyright Kani contributors, Apache or MIT

// run-pass

// revisions: default nomiropt
//[nomiropt]compile-flags: -Z mir-opt-level=0

#![feature(generators, generator_trait)]

use std::marker::Unpin;
use std::ops::{Generator, GeneratorState};
use std::pin::Pin;

fn finish<T>(mut amt: usize, mut t: T) -> T::Return
where
    T: Generator<(), Yield = ()> + Unpin,
{
    loop {
        match Pin::new(&mut t).resume(()) {
            GeneratorState::Yielded(()) => amt = amt.checked_sub(1).unwrap(),
            GeneratorState::Complete(ret) => {
                assert_eq!(amt, 0);
                return ret;
            }
        }
    }
}

#[kani::proof]
#[kani::unwind(16)]
fn main() {
    finish(1, || yield);
    finish(8, || {
        for _ in 0..8 {
            yield;
        }
    });
    finish(1, || {
        if true {
            yield;
        } else {
        }
    });
    finish(1, || {
        if false {
        } else {
            yield;
        }
    });
    finish(2, || {
        if {
            yield;
            false
        } {
            yield;
            panic!()
        }
        yield
    });
}