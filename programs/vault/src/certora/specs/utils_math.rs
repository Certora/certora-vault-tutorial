use cvlr::prelude::*;

#[rule]
pub fn mul_div_floor_equiv_ok() {
    let a: u64 = nondet();
    let b: u64 = nondet();
    let c: u64 = nondet();

    let native_res = crate::utils::math::math_native::mul_div_floor(a, b, c).unwrap();
    let certora_res = crate::utils::math::math_certora::mul_div_floor(a, b, c).unwrap();
    cvlr_assert_eq!(native_res, certora_res);
}

#[rule]
pub fn mul_div_floor_equiv_err() {
    let a: u64 = nondet();
    let b: u64 = nondet();
    let c: u64 = nondet();

    let native_res = crate::utils::math::math_native::mul_div_floor(a, b, c);
    let certora_res = crate::utils::math::math_certora::mul_div_floor(a, b, c);
    // -- in logic, division by 0 is an arbitrary output, not an error
    cvlr_assume!(c > 0);
    cvlr_assert_eq!(native_res.is_err(), certora_res.is_err());
}

#[rule]
pub fn mul_div_ceil_equiv_ok() {
    let a: u64 = nondet();
    let b: u64 = nondet();
    let c: u64 = nondet();

    let native_res = crate::utils::math::math_native::mul_div_ceil(a, b, c).unwrap();
    let certora_res = crate::utils::math::math_certora::mul_div_ceil(a, b, c).unwrap();
    cvlr_assert_eq!(native_res, certora_res);
}

#[rule]
pub fn mul_div_ceil_equiv_err() {
    let a: u64 = nondet();
    let b: u64 = nondet();
    let c: u64 = nondet();

    let native_res = crate::utils::math::math_native::mul_div_ceil(a, b, c);
    let certora_res = crate::utils::math::math_certora::mul_div_ceil(a, b, c);
    cvlr_assert_eq!(native_res.is_err(), certora_res.is_err());
}
