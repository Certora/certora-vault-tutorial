//! Harness for Fee-Based Deposits (component of the `certora_vault` program).
//!
//! Three layers of rules:
//!
//!  * *Accounting-core rules* drive the program's own state-transition functions,
//!    `crate::operations::vault_deposit_assets_with_fee` and
//!    `crate::operations::vault_deposit_assets_with_fee_exact`.  These return a
//!    `VaultEffect` whose `assets_to_vault`, `assets_to_fee` and `shares_to_user`
//!    fields are *exactly* the three amounts the handlers hand to the SPL Token
//!    transfer / transfer / mint_to CPIs (see `processor/deposit_with_fee.rs` and
//!    `processor/deposit_with_fee_exact.rs`), so a statement about those fields is
//!    a statement about the CPI amounts.
//!
//!  * *Account-validation rules* drive the program's own
//!    `crate::loaders::DepositWithFeeContext::load`.
//!
//!  * *Guard rules* drive the extracted prefix of each handler,
//!    `crate::processor::deposit_with_fee_load_and_guard` /
//!    `...::deposit_with_fee_exact_load_and_guard`, which is the account load
//!    followed by the self-transfer guard and nothing else.
//!
//! No rule observes a CPI *effect*: the Prover does not model what a cross-program
//! invocation does, so token balances are out of scope throughout.  The two SPL
//! CPI wrappers of the program (`spl_transfer_assets_from_user`, `spl_mint_shares`)
//! are replaced, under this unit's feature, by the inert stand-ins below.

#![allow(unused_imports)]
#![allow(clippy::needless_range_loop)]

use cvlr::mathint::NativeInt;
use cvlr::prelude::*;
use cvlr_solana::{cvlr_deserialize_nondet_accounts, cvlr_nondet_pubkey};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::loaders::DepositWithFeeContext;
use crate::operations::{
    vault_deposit_assets_with_fee, vault_deposit_assets_with_fee_exact, vault_redeem_shares,
    VaultEffect,
};
use crate::state::Vault;

const VAULT_SIZE: usize = core::mem::size_of::<Vault>();
const BPS: u64 = 10_000;

// ---------------------------------------------------------------------------
// CPI stand-ins
// ---------------------------------------------------------------------------
//
// The Prover does not model what a cross-program invocation does, so nothing in
// this batch can be stated about the effect of an SPL transfer or mint.  What the
// real wrappers *do* contribute to a rule is their cost: each builds an
// `spl_token::instruction::*` value (a heap `Vec<AccountMeta>` plus a packed data
// buffer) and then calls `invoke` / `invoke_signed`.  Both are inlined bodily into
// the two handlers, so no summary directive can name them; a `mock_fn` munge
// redirects them here instead.
//
// These stand-ins return `Ok(())` and do nothing else.  Returning success rather
// than an unconstrained `Result` is deliberate: the rules that reach handler code
// assert rejection, and an inert-but-succeeding CPI cannot be the source of that
// rejection — it has to come from the program's own guards.

/// Stand-in for `crate::processor::spl_transfer_assets_from_user`.
pub fn spl_transfer_assets_from_user<'a>(
    _amount: u64,
    _vault_assets: &AccountInfo<'a>,
    _user_assets: &AccountInfo<'a>,
    _mint: &AccountInfo<'a>,
    _authority: &AccountInfo<'a>,
    _spl_token_program: &AccountInfo<'a>,
) -> ProgramResult {
    Ok(())
}

/// Stand-in for `crate::processor::spl_mint_shares`.
pub fn spl_mint_shares<'a>(
    _amount: u64,
    _user_shares_account: &AccountInfo<'a>,
    _mint: &AccountInfo<'a>,
    _spl_token_program: &AccountInfo<'a>,
    _mint_signer_seeds: &[&[u8]],
) -> ProgramResult {
    Ok(())
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// A fully unconstrained `Vault` value.
fn nondet_vault() -> Vault {
    Vault {
        admin: cvlr_nondet_pubkey(),
        slash_admin: cvlr_nondet_pubkey(),
        shares_mint: cvlr_nondet_pubkey(),
        vault_shares_mint_bump: nondet(),
        assets_mint: cvlr_nondet_pubkey(),
        shares: nondet::<u64>().into(),
        assets: nondet::<u64>().into(),
        fee_bps: nondet::<u64>().into(),
        fee_amount: nondet::<u64>().into(),
        fee_token_account: cvlr_nondet_pubkey(),
        vault_assets_account: cvlr_nondet_pubkey(),
        vault_assets_account_bump: nondet(),
    }
}

/// Every field of the zero-copy `Vault` struct compared bit-for-bit.
fn vault_eq(a: &Vault, b: &Vault) -> bool {
    a.admin == b.admin
        && a.slash_admin == b.slash_admin
        && a.shares_mint == b.shares_mint
        && a.assets_mint == b.assets_mint
        && a.fee_token_account == b.fee_token_account
        && a.vault_assets_account == b.vault_assets_account
        && a.vault_shares_mint_bump == b.vault_shares_mint_bump
        && a.vault_assets_account_bump == b.vault_assets_account_bump
        && u64::from(a.shares) == u64::from(b.shares)
        && u64::from(a.assets) == u64::from(b.assets)
        && u64::from(a.fee_bps) == u64::from(b.fee_bps)
        && u64::from(a.fee_amount) == u64::from(b.fee_amount)
}

/// The configuration fields named by property 8.
fn config_eq(a: &Vault, b: &Vault) -> bool {
    a.admin == b.admin
        && a.slash_admin == b.slash_admin
        && a.assets_mint == b.assets_mint
        && a.shares_mint == b.shares_mint
        && a.fee_token_account == b.fee_token_account
        && a.vault_assets_account == b.vault_assets_account
        && u64::from(a.fee_bps) == u64::from(b.fee_bps)
        && a.vault_shares_mint_bump == b.vault_shares_mint_bump
        && a.vault_assets_account_bump == b.vault_assets_account_bump
}

fn store_vault(info: &AccountInfo, v: &Vault) {
    let mut data = info.data.borrow_mut();
    let dst: &mut Vault = bytemuck::from_bytes_mut(&mut data[0..VAULT_SIZE]);
    *dst = *v;
}

fn load_vault(info: &AccountInfo) -> Vault {
    let data = info.data.borrow();
    *bytemuck::from_bytes::<Vault>(&data[0..VAULT_SIZE])
}

/// Pin down the account shapes and key relationships that make
/// `DepositWithFeeContext::load` able to succeed, and write `v` into
/// `accounts[0]`.  `v` is mutated so that its recorded keys agree with the
/// accounts passed in — exactly what `validate()` requires.
fn setup_fee_deposit_accounts(accounts: &[AccountInfo], v: &mut Vault) {
    cvlr_assume!(accounts[0].data_len() == VAULT_SIZE);
    v.assets_mint = *accounts[3].key;
    v.shares_mint = *accounts[4].key;
    v.vault_assets_account = *accounts[1].key;
    v.fee_token_account = *accounts[2].key;
    cvlr_assume!(v.assets_mint != v.shares_mint);
    store_vault(&accounts[0], v);
    cvlr_assume!(accounts[6].is_signer);
    cvlr_assume!(*accounts[8].key == spl_token::id());
}

fn log_effect(e: &VaultEffect) {
    let s = e.shares_to_user;
    let a = e.assets_to_vault;
    let f = e.assets_to_fee;
    clog!(s, a, f);
}

/// `0` if the load+guard prefix accepted, otherwise the `ProgramError::Custom`
/// code (so `VaultError::SelfTransfer` shows as 3), or `999` for a non-custom
/// `ProgramError`.
fn guard_outcome(r: &Result<DepositWithFeeContext<'_>, ProgramError>) -> u64 {
    match r {
        Ok(_) => 0,
        Err(ProgramError::Custom(c)) => *c as u64,
        Err(_) => 999,
    }
}

// ===========================================================================
// Property 1 — fee_is_exact_floor_bps_and_split_sums_to_amount
// ===========================================================================

#[rule]
pub fn rule_fee_is_exact_floor_bps_and_split_sums_to_amount() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let fee_bps = u64::from(v.fee_bps);
    let amount: u64 = nondet();
    clog!(fee_bps, amount);

    let r = vault_deposit_assets_with_fee(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let e = r.unwrap_or_default();
    log_effect(&e);

    let expected_fee =
        NativeInt::from(amount).muldiv(NativeInt::from(fee_bps), NativeInt::from(BPS));
    clog!(expected_fee);

    // the fee CPI amount is exactly floor(amount * fee_bps / 10000)
    cvlr_assert!(NativeInt::from(e.assets_to_fee) == expected_fee);
    // and the two CPI amounts sum to exactly `amount`
    cvlr_assert!(
        NativeInt::from(e.assets_to_vault) + NativeInt::from(e.assets_to_fee)
            == NativeInt::from(amount)
    );
}

// ===========================================================================
// Property 2 — fee_bps_cap_enforced_before_any_effect
// ===========================================================================
//
// Stated at the accounting core, which is where the cap lives: `fee_in_bps()`
// goes through `FeeBps::try_from`, whose `require_le!(value, 10000)` is the only
// place the cap is enforced.  Both handlers call the core *before* any CPI and
// propagate its error with `?` (`processor/deposit_with_fee.rs:33-37`,
// `deposit_with_fee_exact.rs:33-37`), and the core is handed a `RefMut<Vault>`
// aliasing the Vault account's own bytes — so "the core returns Err having written
// nothing to the Vault" is exactly "the instruction aborts with the Vault account
// byte-identical and no transfer or mint issued".
//
// Handler-level versions of these two rules were written and timed out on four
// consecutive prover submissions; see the published commentary.

#[rule]
pub fn rule_fee_bps_cap_enforced_plain() {
    let mut v = nondet_vault();
    let fee_bps = u64::from(v.fee_bps);
    cvlr_assume!(fee_bps > BPS);
    let amount: u64 = nondet();
    clog!(fee_bps, amount);

    let before = v;
    let r = vault_deposit_assets_with_fee(&mut v, amount);
    let is_err = r.is_err();
    let ie = is_err as u64;
    clog!(ie);

    cvlr_assert!(is_err);
    cvlr_assert!(vault_eq(&v, &before));
}

#[rule]
pub fn rule_fee_bps_cap_enforced_exact() {
    let mut v = nondet_vault();
    let fee_bps = u64::from(v.fee_bps);
    cvlr_assume!(fee_bps > BPS);
    let amount: u64 = nondet();
    clog!(fee_bps, amount);

    let before = v;
    let r = vault_deposit_assets_with_fee_exact(&mut v, amount);
    let is_err = r.is_err();
    let ie = is_err as u64;
    clog!(ie);

    cvlr_assert!(is_err);
    cvlr_assert!(vault_eq(&v, &before));
}

// ===========================================================================
// Property 3 — vault_assets_increase_equals_net_transferred_in
// ===========================================================================

#[rule]
pub fn rule_vault_assets_increase_equals_net_transferred_in_plain() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    let assets_before = v.num_assets();
    clog!(assets_before, amount);

    let r = vault_deposit_assets_with_fee(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let e = r.unwrap_or_default();
    log_effect(&e);

    let assets_after = v.num_assets();
    clog!(assets_after);
    cvlr_assert!(
        NativeInt::from(assets_after)
            == NativeInt::from(assets_before) + NativeInt::from(e.assets_to_vault)
    );
}

#[rule]
pub fn rule_vault_assets_increase_equals_net_transferred_in_exact() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    let assets_before = v.num_assets();
    clog!(assets_before, amount);

    let r = vault_deposit_assets_with_fee_exact(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let e = r.unwrap_or_default();
    log_effect(&e);

    let assets_after = v.num_assets();
    clog!(assets_after);
    cvlr_assert!(
        NativeInt::from(assets_after)
            == NativeInt::from(assets_before) + NativeInt::from(e.assets_to_vault)
    );
}

// ===========================================================================
// Property 4 — vault_shares_increase_equals_shares_minted
// ===========================================================================

#[rule]
pub fn rule_vault_shares_increase_equals_shares_minted_plain() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    let shares_before = v.num_shares();
    clog!(shares_before, amount);

    let r = vault_deposit_assets_with_fee(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let e = r.unwrap_or_default();
    log_effect(&e);

    let shares_after = v.num_shares();
    clog!(shares_after);
    cvlr_assert!(
        NativeInt::from(shares_after)
            == NativeInt::from(shares_before) + NativeInt::from(e.shares_to_user)
    );
}

#[rule]
pub fn rule_vault_shares_increase_equals_shares_minted_exact() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    let shares_before = v.num_shares();
    clog!(shares_before, amount);

    let r = vault_deposit_assets_with_fee_exact(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let e = r.unwrap_or_default();
    log_effect(&e);

    let shares_after = v.num_shares();
    clog!(shares_after);
    cvlr_assert!(
        NativeInt::from(shares_after)
            == NativeInt::from(shares_before) + NativeInt::from(e.shares_to_user)
    );
}

// ===========================================================================
// Property 5 — fee_deposit_preserves_solvency
// ===========================================================================

#[rule]
pub fn rule_fee_deposit_preserves_solvency_plain() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    let s0 = v.num_shares();
    let a0 = v.num_assets();
    clog!(s0, a0, amount);

    let r = vault_deposit_assets_with_fee(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let s1 = v.num_shares();
    let a1 = v.num_assets();
    clog!(s1, a1);

    cvlr_assert!(v.is_solvent());
}

#[rule]
pub fn rule_fee_deposit_preserves_solvency_exact() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    let s0 = v.num_shares();
    let a0 = v.num_assets();
    clog!(s0, a0, amount);

    let r = vault_deposit_assets_with_fee_exact(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let s1 = v.num_shares();
    let a1 = v.num_assets();
    clog!(s1, a1);

    cvlr_assert!(v.is_solvent());
}

// ===========================================================================
// Property 6 — fee_deposit_does_not_dilute_existing_holders
// ===========================================================================

#[rule]
pub fn rule_fee_deposit_does_not_dilute_existing_holders_plain() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    let s0 = v.num_shares();
    let a0 = v.num_assets();
    clog!(s0, a0, amount);

    let r = vault_deposit_assets_with_fee(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let s1 = v.num_shares();
    let a1 = v.num_assets();
    clog!(s1, a1);

    // assets_after * shares_before >= assets_before * shares_after
    cvlr_assert!(
        NativeInt::from(a1) * NativeInt::from(s0) >= NativeInt::from(a0) * NativeInt::from(s1)
    );
}

#[rule]
pub fn rule_fee_deposit_does_not_dilute_existing_holders_exact() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    let s0 = v.num_shares();
    let a0 = v.num_assets();
    clog!(s0, a0, amount);

    let r = vault_deposit_assets_with_fee_exact(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let s1 = v.num_shares();
    let a1 = v.num_assets();
    clog!(s1, a1);

    cvlr_assert!(
        NativeInt::from(a1) * NativeInt::from(s0) >= NativeInt::from(a0) * NativeInt::from(s1)
    );
}

// ===========================================================================
// Property 7 — fee_deposit_leaves_fee_amount_counter_unchanged
// ===========================================================================

#[rule]
pub fn rule_fee_deposit_leaves_fee_amount_counter_unchanged_plain() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    let fee_amount_before = v.fee_amount();
    clog!(fee_amount_before, amount);

    let r = vault_deposit_assets_with_fee(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let fee_amount_after = v.fee_amount();
    clog!(fee_amount_after);

    cvlr_assert!(fee_amount_after == fee_amount_before);
}

#[rule]
pub fn rule_fee_deposit_leaves_fee_amount_counter_unchanged_exact() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    let fee_amount_before = v.fee_amount();
    clog!(fee_amount_before, amount);

    let r = vault_deposit_assets_with_fee_exact(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let fee_amount_after = v.fee_amount();
    clog!(fee_amount_after);

    cvlr_assert!(fee_amount_after == fee_amount_before);
}

// ===========================================================================
// Property 8 — fee_deposit_leaves_vault_configuration_fields_unchanged
// ===========================================================================

#[rule]
pub fn rule_fee_deposit_leaves_vault_configuration_fields_unchanged_plain() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    clog!(amount);

    let before = v;
    let r = vault_deposit_assets_with_fee(&mut v, amount);
    cvlr_assume!(r.is_ok());

    cvlr_assert!(config_eq(&v, &before));
}

#[rule]
pub fn rule_fee_deposit_leaves_vault_configuration_fields_unchanged_exact() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    clog!(amount);

    let before = v;
    let r = vault_deposit_assets_with_fee_exact(&mut v, amount);
    cvlr_assume!(r.is_ok());

    cvlr_assert!(config_eq(&v, &before));
}

// ===========================================================================
// Property 9 — fee_deposit_rejects_vault_user_asset_account_aliasing
// ===========================================================================
//
// Driven against the extracted prefix of each handler — the account load followed
// by the self-transfer guard, which is everything the handler does before it
// touches vault state.  Nothing else about the account set is assumed, so the
// rules cover the executions in which validation would otherwise have succeeded
// (and the guard is therefore the only thing that can reject) as well as those in
// which it would not.

#[rule]
pub fn rule_fee_deposit_rejects_aliasing_plain() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    cvlr_assume!(accounts[0].data_len() == VAULT_SIZE);
    // the aliasing case: the vault's asset account *is* the user's asset account
    cvlr_assume!(accounts[1].key == accounts[5].key);

    let r = crate::processor::deposit_with_fee_load_and_guard(&accounts[0..9]);
    let is_err = r.is_err();
    let code = guard_outcome(&r);
    core::mem::forget(r);
    let ie = is_err as u64;
    clog!(ie, code);

    cvlr_assert!(is_err);
    core::mem::forget(accounts);
}

#[rule]
pub fn rule_fee_deposit_rejects_aliasing_exact() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    cvlr_assume!(accounts[0].data_len() == VAULT_SIZE);
    cvlr_assume!(accounts[1].key == accounts[5].key);

    let r = crate::processor::deposit_with_fee_exact_load_and_guard(&accounts[0..9]);
    let is_err = r.is_err();
    let code = guard_outcome(&r);
    core::mem::forget(r);
    let ie = is_err as u64;
    clog!(ie, code);

    cvlr_assert!(is_err);
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 10 — vault_info_owner_and_discriminator_unchecked  (attack vector)
// ===========================================================================

/// `DepositWithFeeContext::load` must reject a `vault_info` account that is not
/// owned by this program.  Expected to FAIL: `VaultInfo::try_from` runs only
/// `Vault::validate` (assets_mint != shares_mint) and never looks at `owner`.
#[rule]
pub fn rule_vault_info_owner_and_discriminator_unchecked() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    let mut v = nondet_vault();
    setup_fee_deposit_accounts(&accounts[..], &mut v);
    cvlr_assume!(accounts[1].key != accounts[5].key);
    // the vault state account is NOT owned by the vault program
    cvlr_assume!(accounts[0].owner != &crate::ID);

    let is_err = { DepositWithFeeContext::load(&accounts[0..9]).is_err() };
    let ie = is_err as u64;
    clog!(ie);
    cvlr_assert!(is_err);
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 11 — fee_deposit_does_not_rederive_vault_pdas  (attack vector)
// ===========================================================================

/// The fee-deposit context must additionally check that `vault_assets_account`
/// is the program address for [VAULT_ASSETS, vault, bump].  Expected to FAIL:
/// `DepositWithFeeContext::validate` only compares against the (unverified)
/// `vault.vault_assets_account` field and never re-derives.
#[rule]
pub fn rule_fee_deposit_does_not_rederive_vault_assets_pda() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    let mut v = nondet_vault();
    setup_fee_deposit_accounts(&accounts[..], &mut v);
    cvlr_assume!(accounts[1].key != accounts[5].key);
    cvlr_assume!(accounts[0].owner == &crate::ID);

    let derived = crate::state::create_vault_assets_account_address(accounts[0].key, &v);
    cvlr_assume!(derived.is_ok());
    let expected = derived.unwrap_or_default();
    // the supplied vault asset account is NOT the canonical vault assets PDA
    cvlr_assume!(accounts[1].key != &expected);

    let is_err = { DepositWithFeeContext::load(&accounts[0..9]).is_err() };
    let ie = is_err as u64;
    clog!(ie);
    cvlr_assert!(is_err);
    core::mem::forget(accounts);
}

/// Same for the shares mint, whose bump is what the `mint_to` CPI is signed with.
#[rule]
pub fn rule_fee_deposit_does_not_rederive_shares_mint_pda() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    let mut v = nondet_vault();
    setup_fee_deposit_accounts(&accounts[..], &mut v);
    cvlr_assume!(accounts[1].key != accounts[5].key);
    cvlr_assume!(accounts[0].owner == &crate::ID);

    let derived = crate::state::create_vault_shares_mint_address(accounts[0].key, &v);
    cvlr_assume!(derived.is_ok());
    let expected = derived.unwrap_or_default();
    cvlr_assume!(accounts[4].key != &expected);

    let is_err = { DepositWithFeeContext::load(&accounts[0..9]).is_err() };
    let ie = is_err as u64;
    clog!(ie);
    cvlr_assert!(is_err);
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 12 — no_user_assets_debited_without_positive_shares
// ===========================================================================

#[rule]
pub fn rule_no_user_assets_debited_without_positive_shares_plain() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    clog!(amount);

    let r = vault_deposit_assets_with_fee(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let e = r.unwrap_or_default();
    log_effect(&e);

    let debited = e.assets_to_vault > 0 || e.assets_to_fee > 0;
    // no debit of the user unless shares are actually minted ...
    cvlr_assert!(!debited || e.shares_to_user > 0);
    // ... and no shares minted unless assets actually enter the vault
    cvlr_assert!(e.shares_to_user == 0 || e.assets_to_vault > 0);
}

#[rule]
pub fn rule_no_user_assets_debited_without_positive_shares_exact() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    clog!(amount);

    let r = vault_deposit_assets_with_fee_exact(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let e = r.unwrap_or_default();
    log_effect(&e);

    let debited = e.assets_to_vault > 0 || e.assets_to_fee > 0;
    cvlr_assert!(!debited || e.shares_to_user > 0);
    cvlr_assert!(e.shares_to_user == 0 || e.assets_to_vault > 0);
}

// ===========================================================================
// Property 13 — exact_fee_deposit_never_over_collects_assets
// ===========================================================================

#[rule]
pub fn rule_exact_fee_deposit_never_over_collects_assets() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    let pre = v;
    clog!(amount);

    let r = vault_deposit_assets_with_fee_exact(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let e = r.unwrap_or_default();
    log_effect(&e);

    // the transfer into the vault is exactly what backs the shares minted,
    // valued at the pre-deposit ratio
    let need = pre.convert_shares_to_assets(e.shares_to_user);
    cvlr_assume!(need.is_ok());
    let need = need.unwrap_or(0);
    clog!(need);
    cvlr_assert!(e.assets_to_vault == need);

    // and the depositor is never charged more than they asked for
    cvlr_assert!(
        NativeInt::from(e.assets_to_vault) + NativeInt::from(e.assets_to_fee)
            <= NativeInt::from(amount)
    );
}

// ===========================================================================
// Property 14 — legitimate_fee_deposit_is_reachable
// ===========================================================================

/// Reachability: a solvent vault with `fee_bps < 10000`, account set accepted by
/// the program's own `DepositWithFeeContext::load`, a signing authority, no
/// self-transfer, and an `amount` for which the accounting core succeeds and
/// mints a strictly positive number of shares.
#[rule]
pub fn rule_legitimate_fee_deposit_is_reachable() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    cvlr_assume!(u64::from(v.fee_bps) < BPS);
    setup_fee_deposit_accounts(&accounts[..], &mut v);
    cvlr_assume!(accounts[0].owner == &crate::ID);
    let not_aliased = accounts[1].key != accounts[5].key;

    let loaded = { DepositWithFeeContext::load(&accounts[0..9]).is_ok() };

    let amount: u64 = nondet();
    let mut v2 = load_vault(&accounts[0]);
    let r = vault_deposit_assets_with_fee(&mut v2, amount);
    let ok = r.is_ok();
    let e = r.unwrap_or_default();
    log_effect(&e);
    let l = loaded as u64;
    let o = ok as u64;
    let na = not_aliased as u64;
    clog!(amount, l, o, na);

    cvlr_satisfy!(loaded && not_aliased && ok && e.shares_to_user > 0);
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 15 — fee_deposit_bricked_when_share_supply_is_zero  (attack vector)
// ===========================================================================

/// For a vault with no outstanding shares but a positive asset balance there
/// must exist an `amount` for which the plain fee deposit succeeds and mints a
/// positive number of shares.  Expected to FAIL: `convert_assets_to_shares`
/// returns floor(net * 0 / assets) = 0, and `Vault::mint_shares` rejects 0.
#[rule]
pub fn rule_fee_deposit_bricked_when_share_supply_is_zero() {
    let mut v = nondet_vault();
    v.shares = 0u64.into();
    cvlr_assume!(v.num_assets() > 0);
    cvlr_assume!(u64::from(v.fee_bps) < BPS);
    let amount: u64 = nondet();
    let a0 = v.num_assets();
    clog!(a0, amount);

    let r = vault_deposit_assets_with_fee(&mut v, amount);
    let ok = r.is_ok();
    let e = r.unwrap_or_default();
    log_effect(&e);
    let o = ok as u64;
    clog!(o);

    cvlr_satisfy!(ok && e.shares_to_user > 0);
}

/// Companion exhibit for the rule above.  A refuted `cvlr_satisfy!` yields no
/// trace, so this states the same claim as an assertion: for *every* `amount`
/// the deposit into a zero-share / positive-asset vault must succeed with a
/// positive share mint.  Expected to FAIL, with a populated counterexample
/// showing the concrete `amount` and the zero share figure.
#[rule]
pub fn rule_fee_deposit_bricked_when_share_supply_is_zero_witness() {
    let mut v = nondet_vault();
    v.shares = 0u64.into();
    cvlr_assume!(v.num_assets() > 0);
    cvlr_assume!(u64::from(v.fee_bps) < BPS);
    let amount: u64 = nondet();
    cvlr_assume!(amount > 0);
    let a0 = v.num_assets();
    let fb = u64::from(v.fee_bps);
    clog!(a0, fb, amount);

    let r = vault_deposit_assets_with_fee(&mut v, amount);
    let ok = r.is_ok();
    let e = r.unwrap_or_default();
    log_effect(&e);
    let o = ok as u64;
    clog!(o);

    cvlr_assert!(ok && e.shares_to_user > 0);
}

// ===========================================================================
// Property 16 — exact_fee_deposit_is_a_refinement_of_plain_fee_deposit
// ===========================================================================

#[rule]
pub fn rule_exact_fee_deposit_is_a_refinement_of_plain_fee_deposit() {
    let v0 = nondet_vault();
    cvlr_assume!(v0.is_solvent());
    let amount: u64 = nondet();
    clog!(amount);

    let mut va = v0;
    let mut vb = v0;

    let ra = vault_deposit_assets_with_fee(&mut va, amount);
    cvlr_assume!(ra.is_ok());
    let ea = ra.unwrap_or_default();
    log_effect(&ea);

    let rb = vault_deposit_assets_with_fee_exact(&mut vb, amount);
    let b_ok = rb.is_ok();
    let eb = rb.unwrap_or_default();
    log_effect(&eb);
    let bo = b_ok as u64;
    clog!(bo);

    // the exact variant is never worse for the depositor
    cvlr_assert!(b_ok);
    cvlr_assert!(eb.shares_to_user == ea.shares_to_user);
    cvlr_assert!(
        NativeInt::from(eb.assets_to_vault) + NativeInt::from(eb.assets_to_fee)
            <= NativeInt::from(ea.assets_to_vault) + NativeInt::from(ea.assets_to_fee)
    );
}

// ===========================================================================
// Property 17 — fee_deposit_shares_are_redeemable_without_profit
// ===========================================================================

#[rule]
pub fn rule_fee_deposit_shares_are_redeemable_without_profit_plain() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    clog!(amount);

    let r = vault_deposit_assets_with_fee(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let e = r.unwrap_or_default();
    log_effect(&e);
    let s = e.shares_to_user;
    let n = e.assets_to_vault;
    cvlr_assume!(s > 0);

    let r2 = vault_redeem_shares(&mut v, s);
    let ok2 = r2.is_ok();
    let e2 = r2.unwrap_or_default();
    let payout = e2.assets_to_user;
    let o2 = ok2 as u64;
    clog!(s, n, payout, o2);

    cvlr_assert!(ok2);
    cvlr_assert!(payout <= n);
}

#[rule]
pub fn rule_fee_deposit_shares_are_redeemable_without_profit_exact() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    clog!(amount);

    let r = vault_deposit_assets_with_fee_exact(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let e = r.unwrap_or_default();
    log_effect(&e);
    let s = e.shares_to_user;
    let n = e.assets_to_vault;
    cvlr_assume!(s > 0);

    let r2 = vault_redeem_shares(&mut v, s);
    let ok2 = r2.is_ok();
    let e2 = r2.unwrap_or_default();
    let payout = e2.assets_to_user;
    let o2 = ok2 as u64;
    clog!(s, n, payout, o2);

    cvlr_assert!(ok2);
    cvlr_assert!(payout <= n);
}

// ===========================================================================
// Property 18 — depositor_receives_full_value_for_net_deposit
// ===========================================================================

#[rule]
pub fn rule_depositor_receives_full_value_for_net_deposit_plain() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    let pre = v;
    clog!(amount);

    let r = vault_deposit_assets_with_fee(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let e = r.unwrap_or_default();
    log_effect(&e);

    let worth = pre.convert_assets_to_shares(e.assets_to_vault);
    cvlr_assume!(worth.is_ok());
    let worth = worth.unwrap_or(0);
    clog!(worth);
    cvlr_assert!(e.shares_to_user >= worth);
}

#[rule]
pub fn rule_depositor_receives_full_value_for_net_deposit_exact() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let amount: u64 = nondet();
    let pre = v;
    clog!(amount);

    let r = vault_deposit_assets_with_fee_exact(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let e = r.unwrap_or_default();
    log_effect(&e);

    let worth = pre.convert_assets_to_shares(e.assets_to_vault);
    cvlr_assume!(worth.is_ok());
    let worth = worth.unwrap_or(0);
    clog!(worth);
    cvlr_assert!(e.shares_to_user >= worth);
}

// ===========================================================================
// Property 19 — exact_fee_never_exceeds_consented_bps_of_requested_amount
// ===========================================================================

#[rule]
pub fn rule_exact_fee_never_exceeds_consented_bps_of_requested_amount() {
    let mut v = nondet_vault();
    cvlr_assume!(v.is_solvent());
    let fee_bps = u64::from(v.fee_bps);
    let amount: u64 = nondet();
    clog!(fee_bps, amount);

    let r = vault_deposit_assets_with_fee_exact(&mut v, amount);
    cvlr_assume!(r.is_ok());
    let e = r.unwrap_or_default();
    log_effect(&e);

    let max_fee = NativeInt::from(amount).muldiv(NativeInt::from(fee_bps), NativeInt::from(BPS));
    clog!(max_fee);
    cvlr_assert!(NativeInt::from(e.assets_to_fee) <= max_fee);
}
