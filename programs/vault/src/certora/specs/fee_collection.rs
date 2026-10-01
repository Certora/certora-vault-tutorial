//! Harness for the **Fee Collection** unit of `certora_vault`.
//!
//! ## What this harness drives
//!
//! The program is a *native* Solana program (not Anchor): each handler takes
//! `&[AccountInfo]` and loads its accounts through a `*Context::load`.
//!
//! `programs/vault/src/processor/collect_fee.rs` holds the handler every
//! property in this batch is about, but `processor/mod.rs` never declared it as
//! a module and the file carries no `use` lines of its own, so
//! `crate::processor::process_collect_fee` did not resolve.  A `redirect_module`
//! munge on `processor` re-mounts the eight developer modules unchanged and adds
//! an inline `collect_fee` module that supplies the five missing imports and
//! `include!`s the developers' file verbatim.  Every rule below therefore drives
//! the program's own handler text.
//!
//! ## The CPI, and how the transfer is observed
//!
//! `spl_transfer_assets_from_vault` (and its three siblings) build a real
//! `spl_token::instruction::*` value whose encoder contains loops the Prover
//! cannot discharge at `loop_iter: 2`.  Each is replaced, under this unit's
//! cargo feature only, by the stand-in of the same name in this module
//! (`mock_fn`).  The stand-in performs **no** CPI and moves **no** tokens — the
//! Prover does not model a CPI's effects in any case — but it *records* what the
//! handler asked for: the amount, the source and destination keys, and the
//! signer seeds.  That record is what makes
//! `collect_fee_transfers_exactly_recorded_fee` statable, and it is also how the
//! attack rules below observe that a payout happened at all.
//!
//! Nothing here asserts over an actual SPL token balance; where a property needs
//! one (property 7) the balance is modelled as a free `u64` constrained exactly
//! as the property describes, which is sound because the handler never reads it.
//!
//! ## Why every rule consumes the handler's `Result` with `.unwrap()`
//!
//! A first submission stated the rejection properties as
//! `cvlr_assert!(handler(..).is_err())`.  All five rules written that way came
//! back TIMEOUT, while the twelve `.unwrap()` rules in the same job finished.
//! Keeping the failure path live merges the whole error-return chain back into
//! the rule and the solver does not finish.  So each property below is stated in
//! the "success implies" direction, which is the contrapositive of the rejection
//! statement and is checked on exactly the executions the handler accepts.

#![allow(unused_imports)]

use cvlr::prelude::*;
use cvlr_solana::cvlr_deserialize_nondet_accounts;
use cvlr_solana::cvlr_nondet_pubkey;
use cvlr_solana::pubkey::Pk;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use crate::state::Vault;

// ===========================================================================
// Recorded CPI effects
//
// `static mut` scalars, written by the transfer stand-in and read by the rules.
// Only place-assignments and copies are performed, so no reference to a mutable
// static is ever taken.
// ===========================================================================

static mut XFER_COUNT: u64 = 0;
static mut XFER_AMOUNT: u64 = 0;
static mut XFER_SRC: Pubkey = Pubkey::new_from_array([0u8; 32]);
static mut XFER_DST: Pubkey = Pubkey::new_from_array([0u8; 32]);
static mut XFER_NSEEDS: u64 = 0;
static mut XFER_SEED_PREFIX_OK: bool = false;
static mut XFER_SEED_VAULT: Pubkey = Pubkey::new_from_array([0u8; 32]);
static mut XFER_SEED_VAULT_OK: bool = false;
static mut XFER_BUMP_OK: bool = false;
static mut XFER_BUMP: u8 = 0;

fn reset_xfer() {
    unsafe {
        XFER_COUNT = 0;
        XFER_AMOUNT = 0;
        XFER_SRC = Pubkey::new_from_array([0u8; 32]);
        XFER_DST = Pubkey::new_from_array([0u8; 32]);
        XFER_NSEEDS = 0;
        XFER_SEED_PREFIX_OK = false;
        XFER_SEED_VAULT = Pubkey::new_from_array([0u8; 32]);
        XFER_SEED_VAULT_OK = false;
        XFER_BUMP_OK = false;
        XFER_BUMP = 0;
    }
}

fn xfer_count() -> u64 {
    unsafe { XFER_COUNT }
}
fn xfer_amount() -> u64 {
    unsafe { XFER_AMOUNT }
}
fn xfer_src() -> Pubkey {
    unsafe { XFER_SRC }
}
fn xfer_dst() -> Pubkey {
    unsafe { XFER_DST }
}
fn xfer_nseeds() -> u64 {
    unsafe { XFER_NSEEDS }
}
fn xfer_seed_prefix_ok() -> bool {
    unsafe { XFER_SEED_PREFIX_OK }
}
fn xfer_seed_vault() -> Pubkey {
    unsafe { XFER_SEED_VAULT }
}
fn xfer_seed_vault_ok() -> bool {
    unsafe { XFER_SEED_VAULT_OK }
}
fn xfer_bump_ok() -> bool {
    unsafe { XFER_BUMP_OK }
}
fn xfer_bump() -> u8 {
    unsafe { XFER_BUMP }
}

// ===========================================================================
// Stand-ins for the program's four SPL-token CPI wrappers
// (`crate::processor::spl_token_utils::*`), installed by `mock_fn` under this
// unit's cargo feature.  Signatures are copied from the originals.
// ===========================================================================

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

pub fn spl_mint_shares<'a>(
    _amount: u64,
    _user_shares_account: &AccountInfo<'a>,
    _mint: &AccountInfo<'a>,
    _spl_token_program: &AccountInfo<'a>,
    _mint_signer_seeds: &[&[u8]],
) -> ProgramResult {
    Ok(())
}

pub fn spl_burn_shares<'a>(
    _amount: u64,
    _user_shares_account: &AccountInfo<'a>,
    _mint: &AccountInfo<'a>,
    _authority: &AccountInfo<'a>,
    _spl_token_program: &AccountInfo<'a>,
) -> ProgramResult {
    Ok(())
}

/// Records, rather than performs, the vault-signed transfer.
///
/// `vault_assets` is the *source* and `user_assets` the *destination*, matching
/// the original's parameter order and the `spl_token::instruction::transfer`
/// call it builds.
pub fn spl_transfer_assets_from_vault<'a>(
    amount: u64,
    vault_assets: &AccountInfo<'a>,
    user_assets: &AccountInfo<'a>,
    _mint: &AccountInfo<'a>,
    _spl_token_program: &AccountInfo<'a>,
    vault_assets_signer_seeds: &[&[u8]],
) -> ProgramResult {
    unsafe {
        XFER_COUNT += 1;
        XFER_AMOUNT = amount;
        XFER_SRC = *vault_assets.key;
        XFER_DST = *user_assets.key;
        XFER_NSEEDS = vault_assets_signer_seeds.len() as u64;
    }

    if vault_assets_signer_seeds.len() == 3 {
        // seed 0: the literal b"assets" prefix, compared byte by byte so that no
        // slice-comparison loop enters the encoding.
        let s0 = vault_assets_signer_seeds[0];
        let prefix_ok = s0.len() == 6
            && s0[0] == b'a'
            && s0[1] == b's'
            && s0[2] == b's'
            && s0[3] == b'e'
            && s0[4] == b't'
            && s0[5] == b's';

        // seed 1: the vault state account's key
        let s1 = vault_assets_signer_seeds[1];
        let (vault_seed, vault_seed_ok) = match <[u8; 32]>::try_from(s1) {
            Ok(arr) => (Pubkey::new_from_array(arr), true),
            Err(_) => (Pubkey::new_from_array([0u8; 32]), false),
        };

        // seed 2: the one-byte bump
        let s2 = vault_assets_signer_seeds[2];
        let bump_ok = s2.len() == 1;
        let bump = if bump_ok { s2[0] } else { 0u8 };

        unsafe {
            XFER_SEED_PREFIX_OK = prefix_ok;
            XFER_SEED_VAULT = vault_seed;
            XFER_SEED_VAULT_OK = vault_seed_ok;
            XFER_BUMP_OK = bump_ok;
            XFER_BUMP = bump;
        }
    }

    Ok(())
}

// ===========================================================================
// Account shapes and readers
// ===========================================================================

const VAULT_SIZE: usize = core::mem::size_of::<Vault>();
const TOKEN_ACCOUNT_LEN: usize = spl_token::state::Account::LEN;

/// A nondet account has a nondet data length; pin the vault account's size
/// before `bytemuck` casts it.
fn assume_vault_shape(info: &AccountInfo) {
    cvlr_assume!(info.data_len() == VAULT_SIZE);
}

fn assume_token_shape(info: &AccountInfo) {
    cvlr_assume!(info.data_len() == TOKEN_ACCOUNT_LEN);
}

/// Snapshot of the vault state held in `info`'s data, taken exactly the way
/// `VaultInfo::get` takes it.  `Vault` is `Pod + Copy`, so this is a copy and it
/// holds no borrow on the account.
///
/// The program writes the vault back through `bytemuck::from_bytes_mut` on the
/// account's own buffer, so a post-call snapshot really is the post-state.
fn vault_at(info: &AccountInfo) -> Vault {
    let data = info.try_borrow_data().unwrap();
    *bytemuck::from_bytes::<Vault>(&data[0..VAULT_SIZE])
}

// --- per-handler account setup ---------------------------------------------

/// `CollectFeeContext::load`: 0 vault_info, 1 vault_assets_account,
/// 2 assets_mint, 3 fee_collect_account, 4 authority, 5 spl_token_program.
fn setup_collect_fee<'a>() -> Box<[AccountInfo<'a>; 16]> {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_shape(&accounts[0]);
    accounts
}

/// `DepositContext::load`: 0 vault_info, 1 vault_assets_account, 2 assets_mint,
/// 3 shares_mint, 4 user_assets_account, 5 authority, 6 user_shares_account,
/// 7 spl_token_program
fn setup_deposit<'a>() -> Box<[AccountInfo<'a>; 16]> {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_shape(&accounts[0]);
    accounts
}

/// `DepositWithFeeContext::load`: 0 vault_info, 1 vault_assets_account,
/// 2 vault_fee_account, 3 assets_mint, 4 shares_mint, 5 user_assets_account,
/// 6 authority, 7 user_shares_account, 8 spl_token_program
fn setup_deposit_with_fee<'a>() -> Box<[AccountInfo<'a>; 16]> {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_shape(&accounts[0]);
    accounts
}

/// `RedeemSharesContext::load`: 0 vault_info, 1 vault_assets_account,
/// 2 assets_mint, 3 shares_mint, 4 user_shares_account, 5 authority,
/// 6 user_assets_account, 7 spl_token_program
fn setup_redeem<'a>() -> Box<[AccountInfo<'a>; 16]> {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_shape(&accounts[0]);
    accounts
}

/// `UpdateRewardContext::load`: 0 vault_info, 1 vault_assets_account.
/// The handler unpacks the token account, so its length must be pinned too.
fn setup_update_reward<'a>() -> Box<[AccountInfo<'a>; 16]> {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_shape(&accounts[0]);
    assume_token_shape(&accounts[1]);
    accounts
}

/// `SlashContext::load`: 0 vault_info, 1 vault_assets_account,
/// 2 user_token_account, 3 assets_mint, 4 authority, 5 spl_token_program
fn setup_slash<'a>() -> Box<[AccountInfo<'a>; 16]> {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_shape(&accounts[0]);
    accounts
}

// ===========================================================================
// Shared postconditions
// ===========================================================================

/// Every configuration field named by `vault_config_fields_immutable`.
fn assert_config_immutable(before: &Vault, after: &Vault) {
    let admin_ok = after.admin == before.admin;
    let slash_admin_ok = after.slash_admin == before.slash_admin;
    let assets_mint_ok = after.assets_mint == before.assets_mint;
    let shares_mint_ok = after.shares_mint == before.shares_mint;
    let fee_token_account_ok = after.fee_token_account == before.fee_token_account;
    let vault_assets_account_ok = after.vault_assets_account == before.vault_assets_account;
    let fee_bps_ok = u64::from(after.fee_bps) == u64::from(before.fee_bps);
    let assets_bump_ok = after.vault_assets_account_bump == before.vault_assets_account_bump;
    let shares_bump_ok = after.vault_shares_mint_bump == before.vault_shares_mint_bump;

    clog!(admin_ok, slash_admin_ok, assets_mint_ok, shares_mint_ok);
    clog!(fee_token_account_ok, vault_assets_account_ok, fee_bps_ok);
    clog!(assets_bump_ok, shares_bump_ok);

    cvlr_assert!(admin_ok);
    cvlr_assert!(slash_admin_ok);
    cvlr_assert!(assets_mint_ok);
    cvlr_assert!(shares_mint_ok);
    cvlr_assert!(fee_token_account_ok);
    cvlr_assert!(vault_assets_account_ok);
    cvlr_assert!(fee_bps_ok);
    cvlr_assert!(assets_bump_ok);
    cvlr_assert!(shares_bump_ok);
}

/// `fee_amount_only_cleared_by_collect_fee`, for a handler that is not
/// `process_collect_fee`.
fn assert_fee_amount_not_decreased(before: &Vault, after: &Vault) {
    let fee_before = before.fee_amount();
    let fee_after = after.fee_amount();
    clog!(fee_before, fee_after);
    cvlr_assert!(fee_after >= fee_before);
}

// ===========================================================================
// Property 1: admin_signer_required_for_collect_fee
//
// "may succeed only if the passed `authority` is a signer and its key equals
// the `admin` stored in the vault at entry" — stated as success-implies, which
// is the contrapositive of "on any other authority it must revert".  The revert
// clauses of the property ("leaving vault.fee_amount unchanged and issuing no
// token transfer") follow from Solana's rollback of a failed instruction.
// ===========================================================================

#[rule]
pub fn rule_admin_signer_required_for_collect_fee() {
    let accounts = setup_collect_fee();
    reset_xfer();

    let vault_before = vault_at(&accounts[0]);
    let admin_at_entry = vault_before.admin;

    crate::processor::process_collect_fee(&accounts[0..6]).unwrap();

    let is_signer = accounts[4].is_signer;
    let is_admin = accounts[4].key == &admin_at_entry;
    let transfers = xfer_count();
    clog!(is_signer, is_admin, transfers);

    cvlr_assert!(is_signer);
    cvlr_assert!(is_admin);

    core::mem::forget(accounts);
}

// ===========================================================================
// Property 2: collect_fee_transfers_exactly_recorded_fee
// ===========================================================================

#[rule]
pub fn rule_collect_fee_transfers_exactly_recorded_fee() {
    let accounts = setup_collect_fee();
    reset_xfer();

    let vault_before = vault_at(&accounts[0]);
    let fee_before = vault_before.fee_amount();
    let recorded_source = vault_before.vault_assets_account;
    let recorded_bump = vault_before.vault_assets_account_bump;
    let destination = *accounts[3].key;
    let vault_key = *accounts[0].key;
    clog!(fee_before, recorded_bump);

    crate::processor::process_collect_fee(&accounts[0..6]).unwrap();

    let count = xfer_count();
    let amount = xfer_amount();
    let source_ok = xfer_src() == recorded_source;
    let dest_ok = xfer_dst() == destination;
    let nseeds = xfer_nseeds();
    let prefix_ok = xfer_seed_prefix_ok();
    let seed_vault_ok = xfer_seed_vault_ok() && xfer_seed_vault() == vault_key;
    let bump_ok = xfer_bump_ok() && xfer_bump() == recorded_bump;
    clog!(count, amount, nseeds);
    clog!(source_ok, dest_ok, prefix_ok);
    clog!(seed_vault_ok, bump_ok);

    // exactly one transfer ...
    cvlr_assert!(count == 1);
    // ... of exactly the fee recorded at entry ...
    cvlr_assert!(amount == fee_before);
    // ... out of the vault's recorded asset account ...
    cvlr_assert!(source_ok);
    // ... into the passed fee collection account ...
    cvlr_assert!(dest_ok);
    // ... signed with [VAULT_ASSETS, vault_info.key, vault.vault_assets_account_bump]
    cvlr_assert!(nseeds == 3);
    cvlr_assert!(prefix_ok);
    cvlr_assert!(seed_vault_ok);
    cvlr_assert!(bump_ok);

    core::mem::forget(accounts);
}

// ===========================================================================
// Property 3: collect_fee_zeroes_fee_amount
// Property 6 (the collect_fee half): it decreases exactly to zero.
// ===========================================================================

#[rule]
pub fn rule_collect_fee_zeroes_fee_amount() {
    let accounts = setup_collect_fee();
    reset_xfer();

    let fee_before = vault_at(&accounts[0]).fee_amount();
    clog!(fee_before);

    crate::processor::process_collect_fee(&accounts[0..6]).unwrap();

    let fee_after = vault_at(&accounts[0]).fee_amount();
    let paid = xfer_amount();
    clog!(fee_after, paid);

    cvlr_assert!(fee_after == 0);
    // the drop is exactly what was paid out: nothing is silently destroyed
    cvlr_assert!(fee_before - fee_after == paid);

    core::mem::forget(accounts);
}

// ===========================================================================
// Property 4: collect_fee_preserves_assets_and_shares
// ===========================================================================

#[rule]
pub fn rule_collect_fee_preserves_assets_and_shares() {
    let accounts = setup_collect_fee();
    reset_xfer();

    let before = vault_at(&accounts[0]);
    let assets_before = before.num_assets();
    let shares_before = before.num_shares();
    let solvent_before = before.is_solvent();
    clog!(assets_before, shares_before, solvent_before);

    crate::processor::process_collect_fee(&accounts[0..6]).unwrap();

    let after = vault_at(&accounts[0]);
    let assets_after = after.num_assets();
    let shares_after = after.num_shares();
    let solvent_after = after.is_solvent();
    clog!(assets_after, shares_after, solvent_after);

    cvlr_assert!(assets_after == assets_before);
    cvlr_assert!(shares_after == shares_before);
    cvlr_assert!(!solvent_before || solvent_after);

    core::mem::forget(accounts);
}

// ===========================================================================
// Property 5: vault_config_fields_immutable        (all eight handlers)
// Property 6: fee_amount_only_cleared_by_collect_fee (the seven others)
// ===========================================================================

#[rule]
pub fn rule_vault_invariants_preserved_deposit() {
    let accounts = setup_deposit();
    reset_xfer();
    let amount: u64 = nondet();
    clog!(amount);
    let before = vault_at(&accounts[0]);
    crate::processor::process_deposit(&accounts[0..8], amount).unwrap();
    let after = vault_at(&accounts[0]);
    assert_config_immutable(&before, &after);
    assert_fee_amount_not_decreased(&before, &after);
    core::mem::forget(accounts);
}

#[rule]
pub fn rule_vault_invariants_preserved_deposit_exact() {
    let accounts = setup_deposit();
    reset_xfer();
    let amount: u64 = nondet();
    clog!(amount);
    let before = vault_at(&accounts[0]);
    crate::processor::process_deposit_exact(&accounts[0..8], amount).unwrap();
    let after = vault_at(&accounts[0]);
    assert_config_immutable(&before, &after);
    assert_fee_amount_not_decreased(&before, &after);
    core::mem::forget(accounts);
}

#[rule]
pub fn rule_vault_invariants_preserved_deposit_with_fee() {
    let accounts = setup_deposit_with_fee();
    reset_xfer();
    let amount: u64 = nondet();
    clog!(amount);
    let before = vault_at(&accounts[0]);
    crate::processor::process_deposit_with_fee(&accounts[0..9], amount).unwrap();
    let after = vault_at(&accounts[0]);
    assert_config_immutable(&before, &after);
    assert_fee_amount_not_decreased(&before, &after);
    core::mem::forget(accounts);
}

#[rule]
pub fn rule_vault_invariants_preserved_deposit_with_fee_exact() {
    let accounts = setup_deposit_with_fee();
    reset_xfer();
    let amount: u64 = nondet();
    clog!(amount);
    let before = vault_at(&accounts[0]);
    crate::processor::process_deposit_with_fee_exact(&accounts[0..9], amount).unwrap();
    let after = vault_at(&accounts[0]);
    assert_config_immutable(&before, &after);
    assert_fee_amount_not_decreased(&before, &after);
    core::mem::forget(accounts);
}

#[rule]
pub fn rule_vault_invariants_preserved_redeem_shares() {
    let accounts = setup_redeem();
    reset_xfer();
    let amount: u64 = nondet();
    clog!(amount);
    let before = vault_at(&accounts[0]);
    crate::processor::process_redeem_shares(&accounts[0..8], amount).unwrap();
    let after = vault_at(&accounts[0]);
    assert_config_immutable(&before, &after);
    assert_fee_amount_not_decreased(&before, &after);
    core::mem::forget(accounts);
}

#[rule]
pub fn rule_vault_invariants_preserved_update_reward() {
    let accounts = setup_update_reward();
    reset_xfer();
    let before = vault_at(&accounts[0]);
    crate::processor::process_update_reward(&accounts[0..2]).unwrap();
    let after = vault_at(&accounts[0]);
    assert_config_immutable(&before, &after);
    assert_fee_amount_not_decreased(&before, &after);
    core::mem::forget(accounts);
}

#[rule]
pub fn rule_vault_invariants_preserved_slash() {
    let accounts = setup_slash();
    reset_xfer();
    let amount: u64 = nondet();
    clog!(amount);
    let before = vault_at(&accounts[0]);
    crate::processor::process_slash(&accounts[0..6], amount).unwrap();
    let after = vault_at(&accounts[0]);
    assert_config_immutable(&before, &after);
    assert_fee_amount_not_decreased(&before, &after);
    core::mem::forget(accounts);
}

/// The collect-fee half of `vault_config_fields_immutable`.
#[rule]
pub fn rule_collect_fee_config_immutable() {
    let accounts = setup_collect_fee();
    reset_xfer();
    let before = vault_at(&accounts[0]);
    crate::processor::process_collect_fee(&accounts[0..6]).unwrap();
    let after = vault_at(&accounts[0]);
    assert_config_immutable(&before, &after);
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 7: collect_fee_must_not_spend_share_backing_assets   (attack vector)
//
// The vault asset token account's balance is modelled as a free `u64` — sound
// because the handler never reads it, and because the CPI stand-in moves
// nothing.  It is constrained exactly as the property describes: it covers the
// recorded assets but not the recorded assets plus the recorded fee.
//
// The postcondition is that what the handler asks to be transferred out still
// leaves the recorded assets backed.
// ===========================================================================

#[rule]
pub fn rule_collect_fee_must_not_spend_share_backing_assets() {
    let accounts = setup_collect_fee();
    reset_xfer();

    let before = vault_at(&accounts[0]);
    let assets = before.num_assets();
    let fee = before.fee_amount();
    let balance: u64 = nondet();

    // a state the program itself can hold: `Vault::gross_assets()` would panic
    // otherwise, so this keeps the reported witness realistic
    cvlr_assume!((assets as u128) + (fee as u128) <= u64::MAX as u128);
    // the token account backs the recorded assets, but not the recorded fee
    cvlr_assume!(balance >= assets);
    cvlr_assume!((balance as u128) < (assets as u128) + (fee as u128));
    clog!(balance, assets, fee);

    crate::processor::process_collect_fee(&accounts[0..6]).unwrap();

    let paid = xfer_amount();
    let assets_after = vault_at(&accounts[0]).num_assets();
    clog!(paid, assets_after);

    cvlr_assert!((paid as u128) + (assets_after as u128) <= balance as u128);

    core::mem::forget(accounts);
}

// ===========================================================================
// Property 8: collect_fee_requires_program_owned_vault   (attack vector)
//
// Stated as "success implies no payout": if the vault state account is not
// owned by this program, then no execution the handler accepts may issue a
// transfer out of the vault's asset account.
// ===========================================================================

#[rule]
pub fn rule_collect_fee_requires_program_owned_vault() {
    let accounts = setup_collect_fee();
    reset_xfer();

    cvlr_assume!(accounts[0].owner != &crate::ID);
    let fee_before = vault_at(&accounts[0]).fee_amount();
    cvlr_assume!(fee_before > 0);
    clog!(fee_before);

    crate::processor::process_collect_fee(&accounts[0..6]).unwrap();

    let transfers = xfer_count();
    let paid = xfer_amount();
    let fee_after = vault_at(&accounts[0]).fee_amount();
    clog!(transfers, paid, fee_after);

    // an account this program does not own is not vault state; the handler must
    // not have acted on it
    cvlr_assert!(transfers == 0);

    core::mem::forget(accounts);
}

// ===========================================================================
// Property 9: collect_fee_requires_derived_vault_assets_pda   (attack vector)
// ===========================================================================

#[rule]
pub fn rule_collect_fee_requires_derived_vault_assets_pda() {
    let accounts = setup_collect_fee();
    reset_xfer();

    let before = vault_at(&accounts[0]);
    let fee_before = before.fee_amount();
    cvlr_assume!(fee_before > 0);

    // the re-derivation `DepositContext`/`SlashContext` perform through
    // `VaultAssetsAccount::check_vault`, and `CollectFeeContext` omits
    let derived = crate::state::create_vault_assets_account_address(accounts[0].key, &before);
    cvlr_assume!(derived.is_ok());
    let derived = derived.unwrap();
    let is_derived_pda = accounts[1].key == &derived;
    cvlr_assume!(!is_derived_pda);
    clog!(fee_before, is_derived_pda);

    crate::processor::process_collect_fee(&accounts[0..6]).unwrap();

    let transfers = xfer_count();
    let paid = xfer_amount();
    clog!(transfers, paid);

    // the handler signs the transfer with PDA(VAULT_ASSETS, vault, bump); if the
    // source is not that PDA the two are not tied together, so the handler must
    // not have issued the transfer
    cvlr_assert!(transfers == 0);

    core::mem::forget(accounts);
}

// ===========================================================================
// Property 10: collect_fee_rejects_self_transfer   (attack vector)
// ===========================================================================

#[rule]
pub fn rule_collect_fee_rejects_self_transfer() {
    let accounts = setup_collect_fee();
    reset_xfer();

    // fee_collect_account == vault_assets_account
    cvlr_assume!(accounts[3].key == accounts[1].key);

    let fee_before = vault_at(&accounts[0]).fee_amount();
    cvlr_assume!(fee_before > 0);
    clog!(fee_before);

    crate::processor::process_collect_fee(&accounts[0..6]).unwrap();

    let transfers = xfer_count();
    let self_transfer = transfers > 0 && xfer_src() == xfer_dst();
    let fee_after = vault_at(&accounts[0]).fee_amount();
    clog!(transfers, self_transfer, fee_after);

    // the guard `process_deposit_with_fee` has and this handler lacks: a
    // self-transfer moves nothing while the fee claim is zeroed
    cvlr_assert!(!self_transfer);

    core::mem::forget(accounts);
}

// ===========================================================================
// Property 11: collect_fee_reachable_for_admin
//
// An existence claim, so `cvlr_satisfy!` is the right quantifier: with the
// recorded admin signing, a program-owned and internally consistent vault, the
// correctly derived vault assets account and `fee_amount > 0`, there is an
// execution in which the instruction succeeds, the fee is paid out and the
// counter is cleared.  `.unwrap()` prunes the failing executions, so reaching
// the `satisfy` is exactly "the handler succeeded".
// ===========================================================================

#[rule]
pub fn rule_collect_fee_reachable_for_admin() {
    let accounts = setup_collect_fee();
    reset_xfer();

    let before = vault_at(&accounts[0]);

    // program-owned, internally consistent vault with a positive recorded fee
    cvlr_assume!(accounts[0].owner == &crate::ID);
    cvlr_assume!(before.assets_mint != before.shares_mint);
    cvlr_assume!(before.fee_amount() > 0);
    // the recorded admin signs
    cvlr_assume!(accounts[4].is_signer);
    cvlr_assume!(accounts[4].key == &before.admin);
    // the passed vault assets account really is the derived PDA ...
    let derived = crate::state::create_vault_assets_account_address(accounts[0].key, &before);
    cvlr_assume!(derived.is_ok());
    cvlr_assume!(accounts[1].key == &derived.unwrap());
    // ... and it is not the destination
    cvlr_assume!(accounts[3].key != accounts[1].key);

    crate::processor::process_collect_fee(&accounts[0..6]).unwrap();

    let fee_after = vault_at(&accounts[0]).fee_amount();
    let paid = xfer_amount();
    let transfers = xfer_count();
    clog!(fee_after, paid, transfers);

    cvlr_satisfy!(transfers == 1 && paid > 0 && fee_after == 0);

    core::mem::forget(accounts);
}
