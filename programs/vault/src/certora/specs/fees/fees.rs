use crate::certora::specs::base::{base_deposit_assets, base_deposit_assets_with_fee, CvlrProp};
use crate::certora::specs::fees::props::FeeAssessedProp;
use crate::certora::specs::solvency::props::SolvencyInvariant;
use crate::operations::{vault_deposit_assets, vault_deposit_assets_with_fee};
use crate::state::Vault;
use cvlr::mathint::NativeInt;
use cvlr::prelude::*;

#[rule]
pub fn rule_fees_assessed_deposit_assets_with_fee() {
    base_deposit_assets_with_fee::<FeeAssessedProp>();
}

#[rule]
pub fn rule_fees_assessed_deposit_assets() {
    base_deposit_assets::<FeeAssessedProp>();
}

fn safe_assumptions(vault: &Vault, token_amount: u64) {
    // vault is solvent. We have proved separately that solvency is an invariant.
    // Thus, it is safe to use it as an assumption.
    let solvency = SolvencyInvariant::new(&vault);
    solvency.assume_pre();

    // `token_amount` and `vault.num_assets()` have the same mint so the sum cannot overflow
    // because mint's supply is u64.
    let sum = NativeInt::from(token_amount) + NativeInt::from(vault.num_assets());
    cvlr_assume!(sum.is_u64());
}

#[rule]
/// `deposit_assets_with_fees` always succeeds
pub fn rule_liveness_deposit_assets_with_fee() {
    let mut vault: Vault = nondet();
    let token_amount: u64 = nondet();
    clog!(token_amount, vault);

    safe_assumptions(&vault, token_amount);

    let effect =
        vault_deposit_assets_with_fee(&mut vault, token_amount).inspect(|effect| clog!(effect));

    cvlr_assert!(effect.is_ok());
}

#[rule]
/// If `deposit_assets_with_fee` and `deposit_assets` succeed then their effects are the same.
pub fn rule_equivalence_deposit_with_fees_and_feeless_ok() {
    let token_amount = nondet();

    let mut vault_fees: Vault = nondet();
    let mut vault_feeless: Vault = vault_fees.clone();

    cvlr_assume!(vault_fees.fee_in_bps().unwrap().is_zero());

    let effect_fees = vault_deposit_assets_with_fee(&mut vault_fees, token_amount).unwrap();
    let effect_feeless = vault_deposit_assets(&mut vault_feeless, token_amount).unwrap();

    clog!(vault_fees, vault_feeless, effect_fees, effect_feeless);
    cvlr_assert_eq!(effect_fees, effect_feeless);
}

#[rule]
/// One cannot return error and the other ok and if both return error then the error code is the same.
pub fn rule_equivalence_deposit_with_fees_and_feeless_err() {
    let token_amount = nondet();

    let mut vault_fees: Vault = nondet();
    let mut vault_feeless: Vault = vault_fees.clone();

    cvlr_assume!(vault_fees.fee_in_bps().unwrap().is_zero());

    let effect_fees = vault_deposit_assets_with_fee(&mut vault_fees, token_amount);
    let effect_feeless = vault_deposit_assets(&mut vault_feeless, token_amount);

    // both ok or both error
    cvlr_assert_eq!(effect_fees.is_err(), effect_feeless.is_err());

    // if both error then both same error code
    if let (Err(err_fees), Err(err_feeless)) = (effect_fees, effect_feeless) {
        cvlr_assert_eq!(err_fees, err_feeless);
    }
}
