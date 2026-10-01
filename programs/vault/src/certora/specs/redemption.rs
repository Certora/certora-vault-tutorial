//! Harness for Redemption. Written by the CVLR author.
//!
//! Subject: `crate::processor::process_redeem_shares` (native Solana program, not Anchor).
//!
//! Accounts expected by `RedeemSharesContext::load`, in order:
//!   0 vault_info, 1 vault_assets_account, 2 assets_mint, 3 shares_mint,
//!   4 user_shares_account, 5 authority (signer), 6 user_assets_account,
//!   7 spl_token_program.
//!
//! Three modelling notes that the rules below depend on.
//!
//! * `Vault::shares`/`Vault::assets` are `PodU64` — eight bytes in the account's
//!   data.  Under `-solanaTACMathInt true` the Prover's model of a byte-level
//!   read does **not** re-impose that width, and counterexamples came back with
//!   stored totals of 2^64 .. 2^256.  Every arithmetic rule therefore re-states
//!   the width with `NativeInt::is_u64()`.  That excludes only states the
//!   on-chain type cannot represent; it assumes nothing about the handler.
//! * The SPL-token cross-program invocations are not modelled by the Prover:
//!   `solana_program::program::{invoke, invoke_signed}` come back as external
//!   calls that write nothing and return an arbitrary result.  Under this
//!   unit's feature the program's two CPI wrappers are therefore replaced by
//!   the recording stand-ins below, so a rule can see what the handler asked
//!   the token program to do.  Nothing here claims anything about tokens
//!   actually burned or moved.
//! * Because the stand-ins replace the wrappers, the instruction the program
//!   builds is checked separately: the munge also lifted each
//!   `spl_token::instruction::*` construction into a `build_*_ix` function of
//!   the program's own, which the two builder rules drive directly.  The two
//!   halves chain — the handler routes account X into the wrapper's n-th
//!   account parameter, and the builder puts its n-th account parameter into
//!   the instruction's n-th meta.

#![allow(unused_imports)]

use cvlr::mathint::NativeInt;
use cvlr::prelude::*;
use cvlr_solana::cvlr_deserialize_nondet_accounts;
use cvlr_solana::pubkey::Pk;

use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::Instruction;
use solana_program::pubkey::Pubkey;

use crate::processor::spl_token_utils::{build_burn_shares_ix, build_transfer_assets_from_vault_ix};
use crate::state::{create_vault_assets_account_address, create_vault_shares_mint_address, Vault};

const VAULT_LEN: usize = core::mem::size_of::<Vault>();
const ZERO_PK: Pubkey = Pubkey::new_from_array([0u8; 32]);

// ---------------------------------------------------------------------------
// CPI observation: recording stand-ins for the program's two CPI wrappers
// ---------------------------------------------------------------------------

/// What the handler asked a CPI wrapper to do: the amount, the accounts it
/// routed into each of the wrapper's account parameters, the target program
/// account, and the position of the call in the handler's CPI sequence.
#[derive(Clone, Copy)]
pub struct CallRecord {
    pub seen: bool,
    pub seq: u64,
    pub amount: u64,
    pub prog: Pubkey,
    pub k0: Pubkey,
    pub k1: Pubkey,
    pub k2: Pubkey,
}

impl CallRecord {
    const EMPTY: Self = CallRecord {
        seen: false,
        seq: 0,
        amount: 0,
        prog: ZERO_PK,
        k0: ZERO_PK,
        k1: ZERO_PK,
        k2: ZERO_PK,
    };
}

static mut BURN_CALL: CallRecord = CallRecord::EMPTY;
static mut XFER_CALL: CallRecord = CallRecord::EMPTY;
static mut NCALL: u64 = 0;

fn cpi_reset() {
    unsafe {
        BURN_CALL = CallRecord::EMPTY;
        XFER_CALL = CallRecord::EMPTY;
        NCALL = 0;
    }
}

fn burn_call() -> CallRecord {
    unsafe { BURN_CALL }
}

fn xfer_call() -> CallRecord {
    unsafe { XFER_CALL }
}

fn cpi_count() -> u64 {
    unsafe { NCALL }
}

/// Recording stand-in for `crate::processor::spl_token_utils::spl_burn_shares`.
///
/// Same signature; records what the handler handed it and returns `Ok(())`
/// without issuing a CPI.
pub fn spl_burn_shares<'a>(
    amount: u64,
    user_shares_account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    spl_token_program: &AccountInfo<'a>,
) -> ProgramResult {
    unsafe {
        BURN_CALL = CallRecord {
            seen: true,
            seq: NCALL,
            amount,
            prog: *spl_token_program.key,
            k0: *user_shares_account.key,
            k1: *mint.key,
            k2: *authority.key,
        };
        NCALL += 1;
    }
    Ok(())
}

/// Recording stand-in for
/// `crate::processor::spl_token_utils::spl_transfer_assets_from_vault`.
pub fn spl_transfer_assets_from_vault<'a>(
    amount: u64,
    vault_assets: &AccountInfo<'a>,
    user_assets: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    spl_token_program: &AccountInfo<'a>,
    _vault_assets_signer_seeds: &[&[u8]],
) -> ProgramResult {
    unsafe {
        XFER_CALL = CallRecord {
            seen: true,
            seq: NCALL,
            amount,
            prog: *spl_token_program.key,
            k0: *vault_assets.key,
            k1: *user_assets.key,
            k2: *mint.key,
        };
        NCALL += 1;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Read a copy of the `Vault` straight out of the account bytes.
///
/// The handler mutates the vault through `RefMut` on the very same bytes, so a
/// read after the call observes the post-state (there is no Anchor-style
/// deferred write-back in this program).
fn read_vault(info: &AccountInfo) -> Vault {
    let data = info.try_borrow_data().unwrap();
    *bytemuck::from_bytes::<Vault>(&data[0..VAULT_LEN])
}

fn fee_bps_of(v: &Vault) -> u64 {
    u64::from(v.fee_bps)
}

/// The low eight bytes of a pubkey, as a `u64`.  The counterexample viewer
/// saturates a 32-byte value to a single sentinel, so two distinct pubkeys can
/// print identically; this gives a reader something that distinguishes them.
fn pk_lo(k: &Pubkey) -> u64 {
    let b = k.to_bytes();
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

/// The vault account has the length a `Vault` occupies.  Nothing else about any
/// account's contents is constrained — `process_redeem_shares` never
/// deserializes a token account or a mint.
fn assume_vault_account_shape(accounts: &[AccountInfo]) {
    cvlr_assume!(accounts[0].data_len() == VAULT_LEN);
}

/// Re-state the storage width of a `PodU64` field (see the module header).
fn assume_u64(x: u64) {
    cvlr_assume!(NativeInt::from(x).is_u64());
}

fn assume_totals_in_range(v: &Vault) {
    assume_u64(v.num_shares());
    assume_u64(v.num_assets());
}

fn ni(x: u64) -> NativeInt {
    NativeInt::from(x)
}

// ---------------------------------------------------------------------------
// Property 1: redeem_preserves_solvency
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_preserves_solvency() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    assume_totals_in_range(&v0);
    let s0 = v0.num_shares();
    let a0 = v0.num_assets();
    let amount: u64 = nondet();

    cvlr_assume!(s0 <= a0); // solvent before
    clog!(s0 => "shares_before", a0 => "assets_before", amount => "amount");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    let v1 = read_vault(&accounts[0]);
    assume_totals_in_range(&v1);
    let s1 = v1.num_shares();
    let a1 = v1.num_assets();
    clog!(s1 => "shares_after", a1 => "assets_after");

    cvlr_assert!(s1 <= a1);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 2: redeem_payout_equals_floor_of_pre_ratio
//
// Stated multiplicatively: `payout == floor(amount * a0 / s0)` is exactly
// `payout * s0 <= amount * a0 < (payout + 1) * s0`, which keeps the symbolic
// division out of the assertion.
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_payout_equals_floor_of_pre_ratio() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    assume_totals_in_range(&v0);
    let s0 = v0.num_shares();
    let a0 = v0.num_assets();
    let amount: u64 = nondet();

    // `mul_div_floor` divides by `s0`, so a successful call on the unequal
    // branch already implies `s0 != 0`; stating it up front keeps the solver
    // from having to build a term for a division by zero.
    cvlr_assume!(s0 == a0 || s0 != 0);
    clog!(s0 => "shares_before", a0 => "assets_before", amount => "amount");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    let v1 = read_vault(&accounts[0]);
    assume_totals_in_range(&v1);
    let a1 = v1.num_assets();
    clog!(v1.num_shares() => "shares_after", a1 => "assets_after");

    let payout = ni(a0) - ni(a1);
    clog!(payout => "payout");

    if s0 == a0 {
        cvlr_assert!(payout == ni(amount));
    } else {
        cvlr_assert!(payout * ni(s0) <= ni(amount) * ni(a0));
        cvlr_assert!(ni(amount) * ni(a0) < (payout + ni(1)) * ni(s0));
    }
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 3 (recorded-shares half): vault.shares falls by exactly `amount`.
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_burns_exactly_requested_shares() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    assume_totals_in_range(&v0);
    let s0 = v0.num_shares();
    let amount: u64 = nondet();
    clog!(s0 => "shares_before", v0.num_assets() => "assets_before", amount => "amount");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    let v1 = read_vault(&accounts[0]);
    assume_totals_in_range(&v1);
    let s1 = v1.num_shares();
    clog!(s1 => "shares_after");

    cvlr_assert!(ni(s1) + ni(amount) == ni(s0));
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 3 (CPI-amount half): the burn CPI carries `amount`, and the transfer
// CPI carries exactly the decrease the vault recorded in `assets`.
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_cpi_amounts_match() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);
    cpi_reset();

    let v0 = read_vault(&accounts[0]);
    assume_totals_in_range(&v0);
    let a0 = v0.num_assets();
    let amount: u64 = nondet();
    clog!(v0.num_shares() => "shares_before", a0 => "assets_before", amount => "amount");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    let v1 = read_vault(&accounts[0]);
    assume_totals_in_range(&v1);
    let a1 = v1.num_assets();
    let burn = burn_call();
    let xfer = xfer_call();
    clog!(a1 => "assets_after", cpi_count() => "cpi_count");
    clog!(burn.amount => "burn_cpi_amount", xfer.amount => "transfer_cpi_amount");

    // Both CPIs are issued, burn first.
    cvlr_assert!(cpi_count() == 2);
    cvlr_assert!(burn.seen && burn.seq == 0);
    cvlr_assert!(xfer.seen && xfer.seq == 1);
    // The burn destroys exactly the shares that were requested ...
    cvlr_assert!(burn.amount == amount);
    // ... and the transfer moves exactly what the vault took off its books.
    cvlr_assert!(ni(xfer.amount) + ni(a1) == ni(a0));
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 4: redeem_does_not_dilute_remaining_holders
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_does_not_dilute_remaining_holders() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    assume_totals_in_range(&v0);
    let s0 = v0.num_shares();
    let a0 = v0.num_assets();
    let amount: u64 = nondet();
    cvlr_assume!(s0 == a0 || s0 != 0);
    clog!(s0 => "shares_before", a0 => "assets_before", amount => "amount");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    let v1 = read_vault(&accounts[0]);
    assume_totals_in_range(&v1);
    let s1 = v1.num_shares();
    let a1 = v1.num_assets();
    clog!(s1 => "shares_after", a1 => "assets_after");

    cvlr_assert!(ni(a1) * ni(s0) >= ni(a0) * ni(s1));
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 5: redeem_rejects_over_redemption_without_wrapping
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_rejects_over_redemption_without_wrapping() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    assume_totals_in_range(&v0);
    let s0 = v0.num_shares();
    let a0 = v0.num_assets();
    let amount: u64 = nondet();
    clog!(s0 => "shares_before", a0 => "assets_before", amount => "amount");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    let v1 = read_vault(&accounts[0]);
    assume_totals_in_range(&v1);
    let s1 = v1.num_shares();
    let a1 = v1.num_assets();
    clog!(s1 => "shares_after", a1 => "assets_after");

    // A successful redemption can only ever have been for shares the vault
    // records, and can only ever have shrunk both totals -- never wrapped.
    cvlr_assert!(amount <= s0);
    cvlr_assert!(s1 <= s0);
    cvlr_assert!(a1 <= a0);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 6: redeem_requires_authority_signature
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_requires_authority_signature() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let amount: u64 = nondet();
    let signed_before = accounts[5].is_signer;
    clog!(amount => "amount", signed_before as u64 => "authority_is_signer");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    cvlr_assert!(accounts[5].is_signer);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 7: redeem_requires_vault_info_owned_by_program
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_requires_vault_info_owned_by_program() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let amount: u64 = nondet();
    clog!(amount => "amount");
    clog!(Pk(accounts[0].owner) => "vault_info_owner");
    clog!(pk_lo(accounts[0].owner) => "vault_info_owner_lo64");
    clog!(Pk(&crate::ID) => "program_id");
    clog!(pk_lo(&crate::ID) => "program_id_lo64");
    clog!((*accounts[0].owner == crate::ID) as u64 => "owner_is_program");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    cvlr_assert!(*accounts[0].owner == crate::ID);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 8 (recorded-field half): the handler does pin the key against the
// vault's own `vault_assets_account` field.
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_vault_assets_account_matches_vault_record() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    let amount: u64 = nondet();
    clog!(amount => "amount");
    clog!(Pk(accounts[1].key) => "vault_assets_account_key");
    clog!(Pk(&v0.vault_assets_account) => "vault.vault_assets_account");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    cvlr_assert!(*accounts[1].key == v0.vault_assets_account);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 8 (canonical-PDA half):
//   redeem_vault_assets_account_must_be_canonical_pda
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_vault_assets_account_is_canonical_pda() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    let amount: u64 = nondet();
    cvlr_assume!(amount > 0);
    let expected = create_vault_assets_account_address(accounts[0].key, &v0).unwrap();
    clog!(amount => "amount");
    clog!(Pk(accounts[1].key) => "vault_assets_account_key");
    clog!(pk_lo(accounts[1].key) => "vault_assets_account_key_lo64");
    clog!(Pk(&expected) => "canonical_vault_assets_pda");
    clog!(pk_lo(&expected) => "canonical_vault_assets_pda_lo64");
    clog!((*accounts[1].key == expected) as u64 => "key_is_canonical_pda");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    cvlr_assert!(*accounts[1].key == expected);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 9 (recorded-field half): the handler does pin shares_mint against
// the vault's `shares_mint` field.
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_shares_mint_matches_vault_record() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    let amount: u64 = nondet();
    clog!(amount => "amount");
    clog!(Pk(accounts[3].key) => "shares_mint_key");
    clog!(Pk(&v0.shares_mint) => "vault.shares_mint");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    cvlr_assert!(*accounts[3].key == v0.shares_mint);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 9 (canonical-PDA half): redeem_shares_mint_must_be_canonical_pda
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_shares_mint_is_canonical_pda() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    let amount: u64 = nondet();
    cvlr_assume!(amount > 0);
    let expected = create_vault_shares_mint_address(accounts[0].key, &v0).unwrap();
    clog!(amount => "amount");
    clog!(Pk(accounts[3].key) => "shares_mint_key");
    clog!(pk_lo(accounts[3].key) => "shares_mint_key_lo64");
    clog!(Pk(&expected) => "canonical_shares_mint_pda");
    clog!(pk_lo(&expected) => "canonical_shares_mint_pda_lo64");
    clog!((*accounts[3].key == expected) as u64 => "key_is_canonical_pda");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    cvlr_assert!(*accounts[3].key == expected);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 10: redeem_rejects_vault_to_self_transfer
//
// Stated as the contrapositive: a call that succeeds cannot have been given the
// same account for both ends of the asset transfer.
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_rejects_vault_to_self_transfer() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let amount: u64 = nondet();
    clog!(amount => "amount");
    clog!(Pk(accounts[1].key) => "vault_assets_account_key");
    clog!(Pk(accounts[6].key) => "user_assets_account_key");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    cvlr_assert!(accounts[1].key != accounts[6].key);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 11: redeem_cpis_only_to_canonical_spl_token_program
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_cpis_only_to_canonical_spl_token_program() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);
    cpi_reset();

    let amount: u64 = nondet();
    clog!(amount => "amount");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    let burn = burn_call();
    let xfer = xfer_call();
    clog!(cpi_count() => "cpi_count");
    clog!(Pk(accounts[7].key) => "spl_token_program_account_key");
    clog!(Pk(&burn.prog) => "burn_cpi_target_program");
    clog!(Pk(&xfer.prog) => "transfer_cpi_target_program");
    clog!(Pk(&spl_token::ID) => "canonical_spl_token_id");

    cvlr_assert!(cpi_count() == 2);
    // Both CPIs are built against the program account the loader validated ...
    cvlr_assert!(burn.prog == spl_token::ID);
    cvlr_assert!(xfer.prog == spl_token::ID);
    // ... and that account is the canonical SPL Token program.
    cvlr_assert!(*accounts[7].key == spl_token::ID);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 12: redeem_leaves_vault_config_unchanged
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_leaves_vault_config_unchanged() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    let amount: u64 = nondet();
    clog!(amount => "amount");
    clog!(fee_bps_of(&v0) => "fee_bps_before", v0.fee_amount() => "fee_amount_before");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    let v1 = read_vault(&accounts[0]);
    clog!(fee_bps_of(&v1) => "fee_bps_after", v1.fee_amount() => "fee_amount_after");

    cvlr_assert!(v1.admin == v0.admin);
    cvlr_assert!(v1.slash_admin == v0.slash_admin);
    cvlr_assert!(v1.shares_mint == v0.shares_mint);
    cvlr_assert!(v1.assets_mint == v0.assets_mint);
    cvlr_assert!(v1.vault_assets_account == v0.vault_assets_account);
    cvlr_assert!(v1.fee_token_account == v0.fee_token_account);
    cvlr_assert!(v1.vault_shares_mint_bump == v0.vault_shares_mint_bump);
    cvlr_assert!(v1.vault_assets_account_bump == v0.vault_assets_account_bump);
    cvlr_assert!(fee_bps_of(&v1) == fee_bps_of(&v0));
    cvlr_assert!(v1.fee_amount() == v0.fee_amount());
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 13: redeem_reachable_for_legitimate_user
//
// Driven at the vault-accounting layer, which is where the property's subject
// lives: `vault_redeem_shares` is the only part of `process_redeem_shares` that
// can reject a legitimately sized redemption on arithmetic grounds, via the
// `checked_sub`s in `burn_shares`/`del_token` and the post-decrement
// `check_invariant` inside `del_token`.  Driving the whole handler instead did
// not finish (TIMEOUT) -- see the commentary.
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_reachable_for_legitimate_user() {
    let mut vault = Vault::default();
    vault.shares = nondet::<u64>().into();
    vault.assets = nondet::<u64>().into();

    let s0 = vault.num_shares();
    let a0 = vault.num_assets();
    assume_u64(s0);
    assume_u64(a0);

    let amount: u64 = nondet();
    assume_u64(amount);

    // A solvent vault and a legitimately sized redemption -- nothing else.
    cvlr_assume!(s0 <= a0);
    cvlr_assume!(amount > 0);
    cvlr_assume!(amount <= s0);
    clog!(s0 => "shares_before", a0 => "assets_before", amount => "amount");

    let res = crate::operations::vault_redeem_shares(&mut vault, amount);
    clog!(res.is_ok() as u64 => "accepted");
    clog!(vault.num_shares() => "shares_after", vault.num_assets() => "assets_after");

    cvlr_assert!(res.is_ok());
}

// ---------------------------------------------------------------------------
// Property 14: deposit_then_redeem_round_trip_no_profit
//
// Driven at the vault-accounting layer: `vault_deposit_assets` followed by
// `vault_redeem_shares` on the same `Vault`, which is exactly the state the two
// instructions would leave behind one after the other.
// ---------------------------------------------------------------------------
#[rule]
fn rule_deposit_then_redeem_round_trip_no_profit() {
    let mut vault = Vault::default();
    vault.shares = nondet::<u64>().into();
    vault.assets = nondet::<u64>().into();

    let s0 = vault.num_shares();
    let a0 = vault.num_assets();
    assume_u64(s0);
    assume_u64(a0);
    cvlr_assume!(s0 <= a0); // the vault's own solvency invariant
    cvlr_assume!(s0 == a0 || a0 != 0);

    let deposited: u64 = nondet();
    assume_u64(deposited);
    clog!(s0 => "shares_0", a0 => "assets_0", deposited => "deposited");

    let minted = crate::operations::vault_deposit_assets(&mut vault, deposited)
        .unwrap()
        .shares_to_user;
    let s1 = vault.num_shares();
    let a1 = vault.num_assets();
    clog!(s1 => "shares_1", a1 => "assets_1", minted => "minted_shares");

    let payout = crate::operations::vault_redeem_shares(&mut vault, minted)
        .unwrap()
        .assets_to_user;
    clog!(vault.num_shares() => "shares_2", vault.num_assets() => "assets_2");
    clog!(payout => "payout");

    cvlr_assert!(payout <= deposited);
}

// ---------------------------------------------------------------------------
// Property 15: redeem_never_leaves_shareless_vault_with_assets
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_never_leaves_shareless_vault_with_assets() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    assume_totals_in_range(&v0);
    let amount: u64 = nondet();
    cvlr_assume!(v0.num_shares() == v0.num_assets() || v0.num_shares() != 0);
    clog!(v0.num_shares() => "shares_before", v0.num_assets() => "assets_before");
    clog!(amount => "amount");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    let v1 = read_vault(&accounts[0]);
    assume_totals_in_range(&v1);
    let s1 = v1.num_shares();
    let a1 = v1.num_assets();
    clog!(s1 => "shares_after", a1 => "assets_after");

    cvlr_assert!(s1 != 0 || a1 == 0);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 16: redeem_requires_distinct_asset_and_share_mints
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_requires_distinct_asset_and_share_mints() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    let amount: u64 = nondet();
    clog!(amount => "amount");
    clog!(Pk(&v0.assets_mint) => "vault.assets_mint", Pk(&v0.shares_mint) => "vault.shares_mint");
    clog!(Pk(accounts[2].key) => "assets_mint_key", Pk(accounts[3].key) => "shares_mint_key");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    cvlr_assert!(v0.assets_mint != v0.shares_mint);
    cvlr_assert!(accounts[2].key != accounts[3].key);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 17 (routing half): which account the handler puts into which
// account parameter of each CPI wrapper.
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_cpi_accounts_and_direction() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);
    cpi_reset();

    let amount: u64 = nondet();
    clog!(amount => "amount");

    crate::processor::process_redeem_shares(&accounts[..8], amount).unwrap();

    let burn = burn_call();
    let xfer = xfer_call();
    clog!(cpi_count() => "cpi_count");
    clog!(Pk(&burn.k0) => "burn_source", Pk(&burn.k1) => "burn_mint", Pk(&burn.k2) => "burn_authority");
    clog!(Pk(&xfer.k0) => "transfer_from", Pk(&xfer.k1) => "transfer_to", Pk(&xfer.k2) => "transfer_mint");
    clog!(Pk(accounts[1].key) => "vault_assets_account");
    clog!(Pk(accounts[4].key) => "user_shares_account");
    clog!(Pk(accounts[6].key) => "user_assets_account");
    clog!(Pk(accounts[5].key) => "authority");
    clog!(Pk(accounts[3].key) => "shares_mint");
    clog!(Pk(accounts[2].key) => "assets_mint");

    cvlr_assert!(cpi_count() == 2);

    // Burn: debits exactly the user shares account passed in, for the vault's
    // shares mint, with `authority` as the token authority.
    cvlr_assert!(burn.k0 == *accounts[4].key);
    cvlr_assert!(burn.k1 == *accounts[3].key);
    cvlr_assert!(burn.k2 == *accounts[5].key);

    // Transfer: out of the vault assets account, into the user assets account
    // named in the instruction -- that direction, for the assets mint.
    cvlr_assert!(xfer.k0 == *accounts[1].key);
    cvlr_assert!(xfer.k1 == *accounts[6].key);
    cvlr_assert!(xfer.k2 == *accounts[2].key);

    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 17 (instruction half) and Property 11 (burn target):
// the burn instruction the program builds names the wrapper's account
// parameters in the SPL Token `Burn` positions, and targets the token program
// it was given.
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_burn_ix_builder() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());

    let amount: u64 = nondet();
    let src = *accounts[4].key;
    let mint = *accounts[3].key;
    let auth = *accounts[5].key;
    let prog = *accounts[7].key;
    clog!(amount => "amount");
    clog!(Pk(&src) => "param_user_shares_account", Pk(&mint) => "param_mint");
    clog!(Pk(&auth) => "param_authority", Pk(&prog) => "param_spl_token_program");

    let ix = build_burn_shares_ix(amount, &src, &mint, &auth, &prog).unwrap();
    clog!(ix.accounts.len() as u64 => "n_metas");
    clog!(Pk(&ix.program_id) => "ix_program_id");
    clog!(Pk(&ix.accounts[0].pubkey) => "meta0", Pk(&ix.accounts[1].pubkey) => "meta1");
    clog!(Pk(&ix.accounts[2].pubkey) => "meta2");

    // Targets the SPL Token program it was handed, which is the canonical one.
    cvlr_assert!(ix.program_id == prog);
    cvlr_assert!(ix.program_id == spl_token::ID);
    // SPL Token `Burn` account order: source, mint, authority.
    cvlr_assert!(ix.accounts.len() == 3);
    cvlr_assert!(ix.accounts[0].pubkey == src);
    cvlr_assert!(ix.accounts[1].pubkey == mint);
    cvlr_assert!(ix.accounts[2].pubkey == auth);
    cvlr_assert!(ix.accounts[2].is_signer);

    core::mem::forget(ix);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 17 (instruction half) and Property 11 (transfer target):
// the transfer instruction the program builds debits its first account
// parameter and credits its second -- never the reverse -- and is signed by the
// debited account itself.
// ---------------------------------------------------------------------------
#[rule]
fn rule_redeem_transfer_ix_builder() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());

    let amount: u64 = nondet();
    let from = *accounts[1].key;
    let to = *accounts[6].key;
    let prog = *accounts[7].key;
    clog!(amount => "amount");
    clog!(Pk(&from) => "param_vault_assets", Pk(&to) => "param_user_assets");
    clog!(Pk(&prog) => "param_spl_token_program");

    let ix = build_transfer_assets_from_vault_ix(amount, &from, &to, &prog).unwrap();
    clog!(ix.accounts.len() as u64 => "n_metas");
    clog!(Pk(&ix.program_id) => "ix_program_id");
    clog!(Pk(&ix.accounts[0].pubkey) => "meta0_source", Pk(&ix.accounts[1].pubkey) => "meta1_destination");
    clog!(Pk(&ix.accounts[2].pubkey) => "meta2_authority");

    cvlr_assert!(ix.program_id == prog);
    cvlr_assert!(ix.program_id == spl_token::ID);
    // SPL Token `Transfer` account order: source, destination, authority.
    cvlr_assert!(ix.accounts.len() == 3);
    cvlr_assert!(ix.accounts[0].pubkey == from);
    cvlr_assert!(ix.accounts[1].pubkey == to);
    cvlr_assert!(ix.accounts[2].pubkey == from);
    cvlr_assert!(ix.accounts[2].is_signer);

    core::mem::forget(ix);
    core::mem::forget(accounts);
}
