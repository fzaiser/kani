// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that the following SIMD intrinsics are supported:
//!  * `simd_and`
//!  * `simd_or`
//!  * `simd_xor`
//! This is done by initializing vectors with the contents of 2-member tuples
//! with symbolic values. The result of using each of the intrinsics is compared
//! against the result of using the associated bitwise operator on the tuples.
#![feature(repr_simd, platform_intrinsics)]

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i8x2(i8, i8);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i16x2(i16, i16);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i32x2(i32, i32);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i64x2(i64, i64);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct u8x2(u8, u8);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct u16x2(u16, u16);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct u32x2(u32, u32);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct u64x2(u64, u64);

extern "platform-intrinsic" {
    fn simd_and<T>(x: T, y: T) -> T;
    fn simd_or<T>(x: T, y: T) -> T;
    fn simd_xor<T>(x: T, y: T) -> T;
}
macro_rules! compare_simd_op_with_normal_op {
    ($simd_op: ident, $normal_op: tt, $simd_type: ident) => {
        let tup_x = (kani::any(), kani::any());
        let tup_y = (kani::any(), kani::any());
        let x = $simd_type(tup_x.0, tup_x.1);
        let y = $simd_type(tup_y.0, tup_y.1);
        let res_and = unsafe { $simd_op(x, y) };
        assert_eq!(tup_x.0 $normal_op tup_y.0, res_and.0);
        assert_eq!(tup_x.1 $normal_op tup_y.1, res_and.1);
    };
}

#[kani::proof]
fn test_simd_and() {
    compare_simd_op_with_normal_op!(simd_and, &, i8x2);
    compare_simd_op_with_normal_op!(simd_and, &, i16x2);
    compare_simd_op_with_normal_op!(simd_and, &, i32x2);
    compare_simd_op_with_normal_op!(simd_and, &, i64x2);
    compare_simd_op_with_normal_op!(simd_and, &, u8x2);
    compare_simd_op_with_normal_op!(simd_and, &, u16x2);
    compare_simd_op_with_normal_op!(simd_and, &, u32x2);
    compare_simd_op_with_normal_op!(simd_and, &, u64x2);
}

#[kani::proof]
fn test_simd_or() {
    compare_simd_op_with_normal_op!(simd_or, |, i8x2);
    compare_simd_op_with_normal_op!(simd_or, |, i16x2);
    compare_simd_op_with_normal_op!(simd_or, |, i32x2);
    compare_simd_op_with_normal_op!(simd_or, |, i64x2);
    compare_simd_op_with_normal_op!(simd_or, |, u8x2);
    compare_simd_op_with_normal_op!(simd_or, |, u16x2);
    compare_simd_op_with_normal_op!(simd_or, |, u32x2);
    compare_simd_op_with_normal_op!(simd_or, |, u64x2);
}

#[kani::proof]
fn test_simd_xor() {
    compare_simd_op_with_normal_op!(simd_xor, ^, i8x2);
    compare_simd_op_with_normal_op!(simd_xor, ^, i16x2);
    compare_simd_op_with_normal_op!(simd_xor, ^, i32x2);
    compare_simd_op_with_normal_op!(simd_xor, ^, i64x2);
    compare_simd_op_with_normal_op!(simd_xor, ^, u8x2);
    compare_simd_op_with_normal_op!(simd_xor, ^, u16x2);
    compare_simd_op_with_normal_op!(simd_xor, ^, u32x2);
    compare_simd_op_with_normal_op!(simd_xor, ^, u64x2);
}
