//! CVLR rules for the **Reward Accounting** component of the vault program.
//!
//! The handler under test is `crate::processor::process_update_reward`, a native
//! (non-Anchor) instruction handler with signature
//! `fn(accounts: &[AccountInfo]) -> ProgramResult` and no non-account arguments:
//! the "new amount" of the component description is *read* from the vault asset
//! token account rather than passed in.
//!
//! Account order expected by `UpdateRewardContext::load`:
//!   0 vault_info (zero-copy `Vault`, writable)
//!   1 vault_assets_account (SPL token account, read-only)
//!
//! Every rule drives the real handler. The only thing the harness models is the
//! *shape* of the two accounts (`data_len`), which a nondeterministic account does
//! not have and without which neither `bytemuck::from_bytes::<Vault>` nor
//! `spl_token::state::Account::unpack` can be analyzed.
//!
//! The post-state is read back out of the account bytes with the same
//! `bytemuck::from_bytes` the program's own `VaultInfo::get` uses, so a write the
//! handler makes through its `RefMut` is visible to the rule (there is no
//! deserialized-copy/write-back indirection here, the state is zero-copy).

#![allow(dead_code)]

use cvlr::mathint::NativeInt;
use cvlr::prelude::*;
use cvlr_solana::cvlr_deserialize_nondet_accounts;
use cvlr_solana::pubkey::Pk;
use solana_program::account_info::AccountInfo;
use solana_program::program_pack::Pack;

use crate::state::Vault;

// ---------------------------------------------------------------------------
// Harness plumbing
// ---------------------------------------------------------------------------

const VAULT_LEN: usize = core::mem::size_of::<Vault>();
const TOKEN_LEN: usize = spl_token::state::Account::LEN;

/// Read a copy of the zero-copy `Vault` out of the account bytes, exactly the way
/// `crate::loaders::VaultInfo::get` does.  The borrow is released on return.
fn read_vault(info: &AccountInfo) -> Vault {
    let data = info.try_borrow_data().unwrap();
    *bytemuck::from_bytes::<Vault>(&data[0..VAULT_LEN])
}

/// The balance the *program* reads out of the vault asset token account — the very
/// function the handler calls, so no separate token model can disagree with it.
///
/// `unwrap` prunes the executions on which the token account does not unpack; on
/// those the handler returns `Err` before touching any state, so every property
/// below holds there trivially.
fn prog_token_amount(info: &AccountInfo) -> u64 {
    crate::processor::spl_token_account_amount(info).unwrap()
}

fn tok_byte(info: &AccountInfo, i: usize) -> u8 {
    let data = info.try_borrow_data().unwrap();
    data[i]
}

fn ni(v: u64) -> NativeInt {
    NativeInt::from(v)
}

/// Every field of the `Vault` state, compared byte-for-byte.
fn vault_fields_eq(x: &Vault, y: &Vault) -> bool {
    x.admin == y.admin
        && x.slash_admin == y.slash_admin
        && x.shares_mint == y.shares_mint
        && x.vault_shares_mint_bump == y.vault_shares_mint_bump
        && x.assets_mint == y.assets_mint
        && u64::from(x.shares) == u64::from(y.shares)
        && u64::from(x.assets) == u64::from(y.assets)
        && u64::from(x.fee_bps) == u64::from(y.fee_bps)
        && u64::from(x.fee_amount) == u64::from(y.fee_amount)
        && x.fee_token_account == y.fee_token_account
        && x.vault_assets_account == y.vault_assets_account
        && x.vault_assets_account_bump == y.vault_assets_account_bump
}

/// The two accounts the handler takes, unconstrained apart from their sizes.
macro_rules! reward_env {
    ($accounts:ident, $a:ident) => {
        let $accounts = Box::new(cvlr_deserialize_nondet_accounts());
        let $a: &[AccountInfo] = &$accounts[..2];
        cvlr_assume!($a[0].data_len() == VAULT_LEN);
        cvlr_assume!($a[1].data_len() == TOKEN_LEN);
        cvlr_assume!($a[0].key != $a[1].key);
    };
}

/// The two account-ownership facts that hold of a real on-chain call: the vault
/// state account belongs to this program, and the asset account belongs to the SPL
/// Token program.  The handler checks *neither* (that is Property 8 / Property 9);
/// assuming them here keeps every counterexample a scenario that can actually
/// happen on chain.
macro_rules! assume_real_owners {
    ($a:ident) => {
        cvlr_assume!(*$a[0].owner == crate::ID);
        cvlr_assume!(*$a[1].owner == spl_token::id());
    };
}

/// Force the vault asset account's bytes into a shape `spl_token`'s `unpack`
/// accepts: no delegate, state = Initialized, not native, no close authority.
/// Only the four `COption` tags and the state byte are pinned — the mint, owner,
/// `amount` and `delegated_amount` stay fully nondeterministic.
///
/// Used only by the acceptance rule, which has to *reach* a successful execution.
fn assume_unpackable_token_account(info: &AccountInfo) {
    let data = info.try_borrow_data().unwrap();
    // delegate: COption<Pubkey> tag at 72..76 == None
    cvlr_assume!(data[72] == 0);
    cvlr_assume!(data[73] == 0);
    cvlr_assume!(data[74] == 0);
    cvlr_assume!(data[75] == 0);
    // state at 108 == AccountState::Initialized
    cvlr_assume!(data[108] == 1);
    // is_native: COption<u64> tag at 109..113 == None
    cvlr_assume!(data[109] == 0);
    cvlr_assume!(data[110] == 0);
    cvlr_assume!(data[111] == 0);
    cvlr_assume!(data[112] == 0);
    // close_authority: COption<Pubkey> tag at 129..133 == None
    cvlr_assume!(data[129] == 0);
    cvlr_assume!(data[130] == 0);
    cvlr_assume!(data[131] == 0);
    cvlr_assume!(data[132] == 0);
}

// ===========================================================================
// Property 1 — update_reward_syncs_assets_exactly_to_token_balance
// ===========================================================================

#[rule]
pub fn rule_update_reward_syncs_assets_to_balance() {
    reward_env!(accounts, a);
    assume_real_owners!(a);
    let v0 = read_vault(&a[0]);
    let (s0, a0) = (v0.num_shares(), v0.num_assets());
    let bal = prog_token_amount(&a[1]);
    clog!(s0, a0, bal);

    crate::processor::process_update_reward(a).unwrap();

    let v1 = read_vault(&a[0]);
    let a1 = v1.num_assets();
    let bal_after = prog_token_amount(&a[1]);
    clog!(a1, bal_after);

    // exactly the token balance — never more (phantom assets), never less
    cvlr_assert!(a1 == bal);
    cvlr_assert!(a1 == bal_after);
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 2 — update_reward_rejects_balance_below_recorded_assets
// ===========================================================================

#[rule]
pub fn rule_update_reward_rejects_balance_below_assets() {
    reward_env!(accounts, a);
    assume_real_owners!(a);
    let v0 = read_vault(&a[0]);
    let bal = prog_token_amount(&a[1]);
    // the token account holds strictly less than the vault believes it does
    cvlr_assume!(bal < v0.num_assets());
    clog!(v0.num_shares(), v0.num_assets(), bal);

    let r = crate::processor::process_update_reward(a);

    let v1 = read_vault(&a[0]);
    let unchanged = vault_fields_eq(&v0, &v1);
    clog!(v1.num_shares(), v1.num_assets());
    clog!(r.is_err(), unchanged);

    cvlr_assert!(r.is_err());
    cvlr_assert!(unchanged);
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 3 — update_reward_never_dilutes_existing_shares
// ===========================================================================

#[rule]
pub fn rule_update_reward_never_dilutes_shares() {
    reward_env!(accounts, a);
    assume_real_owners!(a);
    let v0 = read_vault(&a[0]);
    let (s0, a0) = (v0.num_shares(), v0.num_assets());
    clog!(s0, a0);

    crate::processor::process_update_reward(a).unwrap();

    let v1 = read_vault(&a[0]);
    let (s1, a1) = (v1.num_shares(), v1.num_assets());
    clog!(s1, a1);

    // assets never shrink and the share count is untouched, so assets-per-share
    // is non-decreasing
    cvlr_assert!(a1 >= a0);
    cvlr_assert!(s1 == s0);
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 4 — update_reward_leaves_vault_configuration_untouched
// ===========================================================================

#[rule]
pub fn rule_update_reward_config_untouched() {
    reward_env!(accounts, a);
    assume_real_owners!(a);
    let v0 = read_vault(&a[0]);
    clog!(v0.num_shares(), v0.num_assets(), v0.fee_amount());

    crate::processor::process_update_reward(a).unwrap();

    let v1 = read_vault(&a[0]);
    clog!(v1.num_shares(), v1.num_assets(), v1.fee_amount());
    clog!(u64::from(v1.fee_bps));
    clog!(
        v1.vault_shares_mint_bump as u64,
        v1.vault_assets_account_bump as u64
    );
    clog!(Pk(&v1.admin) => "admin after", Pk(&v1.slash_admin) => "slash_admin after");
    clog!(Pk(&v1.shares_mint) => "shares_mint after", Pk(&v1.assets_mint) => "assets_mint after");
    clog!(Pk(&v1.vault_assets_account) => "vault_assets_account after");
    clog!(Pk(&v1.fee_token_account) => "fee_token_account after");

    cvlr_assert!(v1.admin == v0.admin);
    cvlr_assert!(v1.slash_admin == v0.slash_admin);
    cvlr_assert!(v1.shares_mint == v0.shares_mint);
    cvlr_assert!(v1.assets_mint == v0.assets_mint);
    cvlr_assert!(v1.vault_assets_account == v0.vault_assets_account);
    cvlr_assert!(v1.fee_token_account == v0.fee_token_account);
    cvlr_assert!(v1.vault_shares_mint_bump == v0.vault_shares_mint_bump);
    cvlr_assert!(v1.vault_assets_account_bump == v0.vault_assets_account_bump);
    cvlr_assert!(u64::from(v1.fee_bps) == u64::from(v0.fee_bps));
    cvlr_assert!(v1.fee_amount() == v0.fee_amount());
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 5 — update_reward_performs_no_cpi_and_moves_no_value
//
// A cross-program invocation from this handler could only move lamports or token
// balances, and the only account other than the vault state account it is handed
// is the asset token account.  So the property is observed as: the asset account's
// balance and its other bytes are unchanged, and no account's lamports moved.
// (A token transfer out of the asset account would move its `amount`; a system
// transfer would move lamports.)
// ===========================================================================

#[rule]
pub fn rule_update_reward_moves_no_value() {
    reward_env!(accounts, a);
    assume_real_owners!(a);
    let v0 = read_vault(&a[0]);
    let bal0 = prog_token_amount(&a[1]);
    let lam0_vault = a[0].lamports();
    let lam0_assets = a[1].lamports();
    // three witness bytes outside the `amount` field, one from each end and the
    // `state` byte
    let b0_0 = tok_byte(&a[1], 0);
    let b0_108 = tok_byte(&a[1], 108);
    let b0_164 = tok_byte(&a[1], TOKEN_LEN - 1);
    clog!(v0.num_assets(), bal0, lam0_vault, lam0_assets);

    crate::processor::process_update_reward(a).unwrap();

    let bal1 = prog_token_amount(&a[1]);
    let lam1_vault = a[0].lamports();
    let lam1_assets = a[1].lamports();
    let b1_0 = tok_byte(&a[1], 0);
    let b1_108 = tok_byte(&a[1], 108);
    let b1_164 = tok_byte(&a[1], TOKEN_LEN - 1);
    clog!(bal1, lam1_vault, lam1_assets);
    clog!(b0_0 as u64, b1_0 as u64);
    clog!(b0_108 as u64, b1_108 as u64);
    clog!(b0_164 as u64, b1_164 as u64);

    cvlr_assert!(bal1 == bal0);
    cvlr_assert!(b1_0 == b0_0);
    cvlr_assert!(b1_108 == b0_108);
    cvlr_assert!(b1_164 == b0_164);
    cvlr_assert!(lam1_vault == lam0_vault);
    cvlr_assert!(lam1_assets == lam0_assets);
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 6 — update_reward_preserves_vault_solvency
// ===========================================================================

#[rule]
pub fn rule_update_reward_preserves_solvency() {
    reward_env!(accounts, a);
    assume_real_owners!(a);
    let v0 = read_vault(&a[0]);
    let (s0, a0) = (v0.num_shares(), v0.num_assets());
    // `Vault::is_solvent` held at entry
    cvlr_assume!(s0 <= a0);
    clog!(s0, a0);

    crate::processor::process_update_reward(a).unwrap();

    let v1 = read_vault(&a[0]);
    let (s1, a1) = (v1.num_shares(), v1.num_assets());
    clog!(s1, a1);

    cvlr_assert!(s1 <= a1);
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 7 — update_reward_counts_uncollected_fee_as_reward
//
// `vault_update_reward` subtracts only `vault.assets`, ignoring `vault.fee_amount`,
// yet `collect_fee` pays fees out of the same token account.  The backing invariant
// is therefore `vault.assets + vault.fee_amount <= balance`.
// ===========================================================================

#[rule]
pub fn rule_update_reward_preserves_fee_backing() {
    reward_env!(accounts, a);
    assume_real_owners!(a);
    let v0 = read_vault(&a[0]);
    let (a0, f0) = (v0.num_assets(), v0.fee_amount());
    let bal0 = prog_token_amount(&a[1]);
    // recorded assets *and* the pending fee are fully backed at entry
    cvlr_assume!(ni(a0) + ni(f0) <= ni(bal0));
    clog!(v0.num_shares(), a0, f0, bal0);

    crate::processor::process_update_reward(a).unwrap();

    let v1 = read_vault(&a[0]);
    let (a1, f1) = (v1.num_assets(), v1.fee_amount());
    let bal1 = prog_token_amount(&a[1]);
    clog!(a1, f1, bal1);

    cvlr_assert!(ni(a1) + ni(f1) <= ni(bal1));
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 8 — update_reward_requires_vault_account_owned_by_program
// ===========================================================================

#[rule]
pub fn rule_update_reward_rejects_foreign_owned_vault() {
    reward_env!(accounts, a);
    cvlr_assume!(*a[1].owner == spl_token::id());
    // the "vault" state account is owned by somebody other than this program
    cvlr_assume!(*a[0].owner != crate::ID);
    let v0 = read_vault(&a[0]);
    clog!(v0.num_shares(), v0.num_assets());
    clog!(Pk(a[0].owner) => "vault_info owner", Pk(&crate::ID) => "vault program id");

    let r = crate::processor::process_update_reward(a);

    let v1 = read_vault(&a[0]);
    clog!(v1.num_shares(), v1.num_assets());
    clog!(r.is_err());

    cvlr_assert!(r.is_err());
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 9 — update_reward_must_pin_asset_account_to_canonical_pda_and_token_program
//
// Two rules, one per conjunct of the guard the property asks for.
// ===========================================================================

/// (a) the asset account must be owned by the SPL Token program — otherwise its
/// bytes, and so the balance the handler reads, are attacker-chosen.
#[rule]
pub fn rule_update_reward_rejects_foreign_owned_assets_account() {
    reward_env!(accounts, a);
    cvlr_assume!(*a[0].owner == crate::ID);
    // a token-account-shaped account that is *not* an SPL token account
    cvlr_assume!(*a[1].owner != spl_token::id());
    let v0 = read_vault(&a[0]);
    clog!(v0.num_shares(), v0.num_assets());
    clog!(Pk(a[1].owner) => "assets account owner", Pk(&spl_token::id()) => "spl token program");

    let r = crate::processor::process_update_reward(a);

    let v1 = read_vault(&a[0]);
    clog!(v1.num_shares(), v1.num_assets());
    clog!(r.is_err());

    cvlr_assert!(r.is_err());
    core::mem::forget(accounts);
}

/// (b) the asset account's key must be the canonical
/// `create_program_address([VAULT_ASSETS, vault_pk], vault.vault_assets_account_bump)`
/// — the derivation `VaultAssetsAccount::check_vault` performs for
/// `DepositContext`/`SlashContext` and that `UpdateRewardContext` omits.
///
/// The program's own derivation helper is used, so the address compared against is
/// the one the omitted check would have computed.  (`Pubkey::create_program_address`
/// is summarized by the build, so the value is unconstrained; what the rule shows
/// is that the handler accepts an asset account whose key differs from it, i.e.
/// that no such derivation gates the handler at all.)
#[rule]
pub fn rule_update_reward_rejects_non_canonical_assets_account() {
    reward_env!(accounts, a);
    cvlr_assume!(*a[0].owner == crate::ID);
    cvlr_assume!(*a[1].owner == spl_token::id());
    let v0 = read_vault(&a[0]);
    let derived = crate::state::create_vault_assets_account_address(a[0].key, &v0);
    cvlr_assume!(derived.is_ok());
    let expected = derived.unwrap();
    cvlr_assume!(*a[1].key != expected);
    clog!(v0.num_shares(), v0.num_assets(), v0.vault_assets_account_bump as u64);
    clog!(Pk(a[1].key) => "assets account key", Pk(&expected) => "canonical assets PDA");

    let r = crate::processor::process_update_reward(a);

    let v1 = read_vault(&a[0]);
    clog!(v1.num_shares(), v1.num_assets());
    clog!(r.is_err());

    cvlr_assert!(r.is_err());
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 10 — update_reward_must_not_create_assets_with_zero_shares
//
// Entry state: a vault with no shares and no assets — the state a freshly
// initialized vault is in.  A permissionless call must not be able to leave it
// holding assets with no shares outstanding, because in that state every deposit
// prices at zero shares and `Vault::mint_shares` rejects a zero amount, so the
// vault is bricked and the assets are unredeemable.
// ===========================================================================

#[rule]
pub fn rule_update_reward_no_assets_with_zero_shares() {
    reward_env!(accounts, a);
    assume_real_owners!(a);
    let v0 = read_vault(&a[0]);
    let (s0, a0) = (v0.num_shares(), v0.num_assets());
    cvlr_assume!(s0 == 0);
    cvlr_assume!(a0 == 0);
    let bal0 = prog_token_amount(&a[1]);
    clog!(s0, a0, bal0);

    crate::processor::process_update_reward(a).unwrap();

    let v1 = read_vault(&a[0]);
    let (s1, a1) = (v1.num_shares(), v1.num_assets());
    clog!(s1, a1);

    cvlr_assert!(a1 == 0);
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 11 — update_reward_reachable_without_any_signer
//
// The acceptance half of the component's contract: the permissionless reward path
// is not dead.  Stated universally — for *every* program-owned vault whose
// recorded asset account holds strictly more than the vault records, with no
// signer among the accounts, the handler succeeds and syncs exactly the surplus.
//
// The token account's `COption` tags and state byte are pinned to a shape
// `spl_token`'s `unpack` accepts; its mint, owner field, `amount` and
// `delegated_amount` stay unconstrained.  The asset account's *key* is pinned to
// `vault.vault_assets_account`, which is what the handler checks; its canonicity
// as a PDA cannot be pinned, because the build summarizes
// `Pubkey::create_program_address` and the derived address is unconstrained.
// ===========================================================================

#[rule]
pub fn rule_update_reward_reachable_without_signer() {
    reward_env!(accounts, a);
    assume_real_owners!(a);
    // nobody signed
    cvlr_assume!(!a[0].is_signer);
    cvlr_assume!(!a[1].is_signer);
    assume_unpackable_token_account(&a[1]);

    let v0 = read_vault(&a[0]);
    let (s0, a0) = (v0.num_shares(), v0.num_assets());
    // a well-formed vault (`Vault::validate`) whose recorded asset account is the
    // one being passed (`UpdateRewardContext::validate`)
    cvlr_assume!(v0.assets_mint != v0.shares_mint);
    cvlr_assume!(*a[1].key == v0.vault_assets_account);

    let bal = prog_token_amount(&a[1]);
    // a reward really accrued
    cvlr_assume!(bal > a0);
    clog!(s0, a0, bal);

    let r = crate::processor::process_update_reward(a);

    let v1 = read_vault(&a[0]);
    let (s1, a1) = (v1.num_shares(), v1.num_assets());
    clog!(s1, a1);
    clog!(r.is_ok());

    cvlr_assert!(r.is_ok());
    // ... and assets rose by exactly the surplus
    cvlr_assert!(ni(a1) == ni(a0) + (ni(bal) - ni(a0)));
    cvlr_assert!(a1 == bal);
    core::mem::forget(accounts);
}
