//! Harness for Slashing. Written by the CVLR author.
//!
//! Subject: `crate::processor::process_slash` (native Solana program, not Anchor).
//!
//! Accounts expected by `SlashContext::load`, in order:
//!   0 vault_info, 1 vault_assets_account, 2 user_token_account,
//!   3 assets_mint, 4 authority (signer), 5 spl_token_program.
//!
//! Modelling notes the rules depend on:
//!
//! * `Vault::shares`/`Vault::assets` are `PodU64` — eight bytes in the account's
//!   data.  Under `-solanaTACMathInt true` the Prover's model of a byte-level
//!   read does not re-impose that width, so every arithmetic rule re-states it
//!   with `NativeInt::is_u64()`.  That excludes only states the on-chain type
//!   cannot represent; it assumes nothing about the handler.
//! * The SPL-token cross-program invocation is not modelled by the Prover.
//!   Under this unit's feature the program's `spl_transfer_assets_from_vault`
//!   wrapper is replaced by the recording stand-in below, so a rule can see
//!   what the handler asked the token program to do — including the PDA signer
//!   seeds it passed.  Nothing here claims anything about tokens actually
//!   moved.
//! * Because the stand-in replaces the wrapper, the instruction the program
//!   builds is checked separately: the munge also lifted the
//!   `spl_token::instruction::transfer` construction into
//!   `crate::processor::spl_token_utils::build_transfer_assets_from_vault_ix`,
//!   which one rule drives directly.  The two halves chain — the handler routes
//!   account X into the wrapper's n-th account parameter, and the builder puts
//!   its n-th account parameter into the instruction's n-th meta.
//! * `Pubkey::create_program_address` is summarized: every call returns a fresh
//!   unconstrained address, so the address a *rule* derives and the address the
//!   program derives inside `VaultAssetsAccount::check_vault` are unrelated
//!   values.  Nothing can therefore be stated about acceptance or rejection *by
//!   that check*.  Two earlier rules that tried came back VIOLATED on witnesses
//!   with two different "derived" addresses for the same inputs; they were
//!   removed rather than reported, and the consequences for Property 6 and
//!   Property 9 are spelled out in the commentary.

#![allow(unused_imports)]

use cvlr::mathint::NativeInt;
use cvlr::prelude::*;
use cvlr_solana::cvlr_deserialize_nondet_accounts;
use cvlr_solana::pubkey::Pk;

use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::Instruction;
use solana_program::pubkey::Pubkey;

use crate::processor::spl_token_utils::build_transfer_assets_from_vault_ix;
use crate::state::Vault;

const VAULT_LEN: usize = core::mem::size_of::<Vault>();
const ZERO_PK: Pubkey = Pubkey::new_from_array([0u8; 32]);

// ---------------------------------------------------------------------------
// CPI observation: a recording stand-in for the program's transfer wrapper
// ---------------------------------------------------------------------------

/// What the handler asked `spl_transfer_assets_from_vault` to do: the amount,
/// the accounts routed into each of the wrapper's account parameters, the
/// target program account, and the shape of the PDA signer seeds.
#[derive(Clone, Copy)]
pub struct XferRecord {
    pub seen: bool,
    pub count: u64,
    pub amount: u64,
    pub prog: Pubkey,
    /// wrapper parameter `vault_assets`
    pub src: Pubkey,
    /// wrapper parameter `user_assets`
    pub dst: Pubkey,
    /// wrapper parameter `_mint`
    pub mint: Pubkey,
    pub n_seeds: u64,
    pub seed0_len: u64,
    pub seed1_len: u64,
    pub seed2_len: u64,
    /// the six bytes of seed 0, packed little-endian
    pub seed0_tag: u64,
    /// seed 1 read back as a pubkey
    pub seed1_pk: Pubkey,
    /// the single byte of seed 2
    pub seed2_bump: u8,
}

impl XferRecord {
    const EMPTY: Self = XferRecord {
        seen: false,
        count: 0,
        amount: 0,
        prog: ZERO_PK,
        src: ZERO_PK,
        dst: ZERO_PK,
        mint: ZERO_PK,
        n_seeds: 0,
        seed0_len: 0,
        seed1_len: 0,
        seed2_len: 0,
        seed0_tag: 0,
        seed1_pk: ZERO_PK,
        seed2_bump: 0,
    };
}

static mut XFER_CALL: XferRecord = XferRecord::EMPTY;

fn cpi_reset() {
    unsafe {
        XFER_CALL = XferRecord::EMPTY;
    }
}

fn xfer_call() -> XferRecord {
    unsafe { XFER_CALL }
}

/// The six-byte seed tag, packed little-endian into a `u64`, so a rule can
/// compare seed bytes without a byte-wise loop.
fn tag6(s: &[u8]) -> u64 {
    u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], 0, 0])
}

/// Recording stand-in for
/// `crate::processor::spl_token_utils::spl_transfer_assets_from_vault`.
///
/// Same signature; records what the handler handed it and returns `Ok(())`
/// without issuing a CPI.
pub fn spl_transfer_assets_from_vault<'a>(
    amount: u64,
    vault_assets: &AccountInfo<'a>,
    user_assets: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    spl_token_program: &AccountInfo<'a>,
    vault_assets_signer_seeds: &[&[u8]],
) -> ProgramResult {
    let n = vault_assets_signer_seeds.len();

    let mut l0: u64 = 0;
    let mut l1: u64 = 0;
    let mut l2: u64 = 0;
    let mut tag: u64 = 0;
    let mut pk: Pubkey = ZERO_PK;
    let mut bump: u8 = 0;

    if n > 0 {
        let s = vault_assets_signer_seeds[0];
        l0 = s.len() as u64;
        if s.len() >= 6 {
            tag = tag6(s);
        }
    }
    if n > 1 {
        let s = vault_assets_signer_seeds[1];
        l1 = s.len() as u64;
        if s.len() == 32 {
            let mut a = [0u8; 32];
            a.copy_from_slice(s);
            pk = Pubkey::new_from_array(a);
        }
    }
    if n > 2 {
        let s = vault_assets_signer_seeds[2];
        l2 = s.len() as u64;
        if !s.is_empty() {
            bump = s[0];
        }
    }

    unsafe {
        XFER_CALL = XferRecord {
            seen: true,
            count: XFER_CALL.count + 1,
            amount,
            prog: *spl_token_program.key,
            src: *vault_assets.key,
            dst: *user_assets.key,
            mint: *mint.key,
            n_seeds: n as u64,
            seed0_len: l0,
            seed1_len: l1,
            seed2_len: l2,
            seed0_tag: tag,
            seed1_pk: pk,
            seed2_bump: bump,
        };
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

/// The vault account has the length a `Vault` occupies.  Nothing else about any
/// account's contents is constrained — `process_slash` never deserializes a
/// token account or a mint.
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

// ---------------------------------------------------------------------------
// Property 1: slash_only_by_recorded_slash_admin_signer (acceptance half)
//
// Every execution the handler accepts had `authority` signing and carrying the
// key the pre-state vault records in `slash_admin`.
// ---------------------------------------------------------------------------
#[rule]
fn rule_slash_only_by_recorded_slash_admin_signer() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    let amount: u64 = nondet();
    clog!(amount => "amount");
    clog!(accounts[4].is_signer as u64 => "authority_is_signer");
    clog!(Pk(accounts[4].key) => "authority_key", pk_lo(accounts[4].key) => "authority_key_lo64");
    clog!(Pk(&v0.slash_admin) => "vault.slash_admin", pk_lo(&v0.slash_admin) => "vault.slash_admin_lo64");

    crate::processor::process_slash(&accounts[..6], amount).unwrap();

    cvlr_assert!(accounts[4].is_signer);
    cvlr_assert!(*accounts[4].key == v0.slash_admin);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 1: slash_only_by_recorded_slash_admin_signer (rejection half, a)
//
// A non-signing authority is refused outright: the vault's recorded assets are
// untouched and no asset transfer is requested.
// ---------------------------------------------------------------------------
#[rule]
fn rule_slash_rejects_unsigned_authority() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);
    cpi_reset();

    let v0 = read_vault(&accounts[0]);
    let a0 = v0.num_assets();
    let amount: u64 = nondet();

    cvlr_assume!(!accounts[4].is_signer);
    clog!(amount => "amount", a0 => "assets_before");
    clog!(accounts[4].is_signer as u64 => "authority_is_signer");

    let res = crate::processor::process_slash(&accounts[..6], amount);

    let a1 = read_vault(&accounts[0]).num_assets();
    let x = xfer_call();
    clog!(res.is_ok() as u64 => "accepted", a1 => "assets_after");
    clog!(x.seen as u64 => "transfer_cpi_issued");

    cvlr_assert!(res.is_err());
    cvlr_assert!(a1 == a0);
    cvlr_assert!(!x.seen);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 1: slash_only_by_recorded_slash_admin_signer (rejection half, b)
//
// A signer that is not the recorded slash admin is refused, with the same two
// consequences.
// ---------------------------------------------------------------------------
#[rule]
fn rule_slash_rejects_wrong_authority_key() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);
    cpi_reset();

    let v0 = read_vault(&accounts[0]);
    let a0 = v0.num_assets();
    let amount: u64 = nondet();

    cvlr_assume!(*accounts[4].key != v0.slash_admin);
    clog!(amount => "amount", a0 => "assets_before");
    clog!(accounts[4].is_signer as u64 => "authority_is_signer");
    clog!(Pk(accounts[4].key) => "authority_key", pk_lo(accounts[4].key) => "authority_key_lo64");
    clog!(Pk(&v0.slash_admin) => "vault.slash_admin", pk_lo(&v0.slash_admin) => "vault.slash_admin_lo64");

    let res = crate::processor::process_slash(&accounts[..6], amount);

    let a1 = read_vault(&accounts[0]).num_assets();
    let x = xfer_call();
    clog!(res.is_ok() as u64 => "accepted", a1 => "assets_after");
    clog!(x.seen as u64 => "transfer_cpi_issued");

    cvlr_assert!(res.is_err());
    cvlr_assert!(a1 == a0);
    cvlr_assert!(!x.seen);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 2: slash_decrements_assets_by_exactly_amount
// ---------------------------------------------------------------------------
#[rule]
fn rule_slash_decrements_assets_by_exactly_amount() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);
    cpi_reset();

    let v0 = read_vault(&accounts[0]);
    assume_totals_in_range(&v0);
    let a0 = v0.num_assets();
    let amount: u64 = nondet();
    clog!(v0.num_shares() => "shares_before", a0 => "assets_before", amount => "amount");

    crate::processor::process_slash(&accounts[..6], amount).unwrap();

    let v1 = read_vault(&accounts[0]);
    assume_totals_in_range(&v1);
    let a1 = v1.num_assets();
    let x = xfer_call();
    clog!(a1 => "assets_after", x.amount => "transfer_cpi_amount", x.count => "cpi_count");

    // Exact integers: no wrap in either direction.
    cvlr_assert!(ni(a1) + ni(amount) == ni(a0));
    // Exactly one transfer was requested, and it carries the same `amount`.
    cvlr_assert!(x.seen && x.count == 1);
    cvlr_assert!(x.amount == amount);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 3: slash_preserves_vault_solvency
// ---------------------------------------------------------------------------
#[rule]
fn rule_slash_preserves_vault_solvency() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    assume_totals_in_range(&v0);
    let s0 = v0.num_shares();
    let a0 = v0.num_assets();
    let amount: u64 = nondet();

    cvlr_assume!(s0 <= a0); // solvent before
    clog!(s0 => "shares_before", a0 => "assets_before", amount => "amount");

    crate::processor::process_slash(&accounts[..6], amount).unwrap();

    let v1 = read_vault(&accounts[0]);
    assume_totals_in_range(&v1);
    let s1 = v1.num_shares();
    let a1 = v1.num_assets();
    clog!(s1 => "shares_after", a1 => "assets_after");

    // Still solvent ...
    cvlr_assert!(s1 <= a1);
    // ... which, since shares are untouched and assets fall by `amount`, is the
    // same as saying every accepted amount was within the free (unbacked)
    // balance, with no underflow of the assets counter.
    cvlr_assert!(ni(amount) + ni(s0) <= ni(a0));
    cvlr_assert!(amount <= a0);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 4: slash_modifies_only_assets_field
// ---------------------------------------------------------------------------
#[rule]
fn rule_slash_modifies_only_assets_field() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    let amount: u64 = nondet();
    clog!(amount => "amount");
    clog!(v0.num_shares() => "shares_before", v0.num_assets() => "assets_before");
    clog!(fee_bps_of(&v0) => "fee_bps_before", v0.fee_amount() => "fee_amount_before");

    crate::processor::process_slash(&accounts[..6], amount).unwrap();

    let v1 = read_vault(&accounts[0]);
    clog!(v1.num_shares() => "shares_after", v1.num_assets() => "assets_after");
    clog!(fee_bps_of(&v1) => "fee_bps_after", v1.fee_amount() => "fee_amount_after");

    cvlr_assert!(v1.num_shares() == v0.num_shares());
    cvlr_assert!(v1.admin == v0.admin);
    cvlr_assert!(v1.slash_admin == v0.slash_admin);
    cvlr_assert!(v1.shares_mint == v0.shares_mint);
    cvlr_assert!(v1.assets_mint == v0.assets_mint);
    cvlr_assert!(fee_bps_of(&v1) == fee_bps_of(&v0));
    cvlr_assert!(v1.fee_amount() == v0.fee_amount());
    cvlr_assert!(v1.fee_token_account == v0.fee_token_account);
    cvlr_assert!(v1.vault_assets_account == v0.vault_assets_account);
    cvlr_assert!(v1.vault_assets_account_bump == v0.vault_assets_account_bump);
    cvlr_assert!(v1.vault_shares_mint_bump == v0.vault_shares_mint_bump);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 5: slash_vault_account_owner_unchecked
//
// Stated as the property a correct program would have: a successful
// `process_slash` can only ever have been handed a `vault_info` owned by this
// program.  EXPECTED TO FAIL — `VaultInfo::try_from` never compares
// `info.owner` to `crate::ID`.
// ---------------------------------------------------------------------------
#[rule]
fn rule_slash_requires_vault_info_owned_by_program() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let amount: u64 = nondet();
    clog!(amount => "amount");
    clog!(Pk(accounts[0].owner) => "vault_info_owner");
    clog!(pk_lo(accounts[0].owner) => "vault_info_owner_lo64");
    clog!(Pk(&crate::ID) => "program_id", pk_lo(&crate::ID) => "program_id_lo64");
    clog!((*accounts[0].owner == crate::ID) as u64 => "owner_is_program");

    crate::processor::process_slash(&accounts[..6], amount).unwrap();

    cvlr_assert!(*accounts[0].owner == crate::ID);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 6: slash_assets_account_must_match_recorded_pubkey
//
// `SlashContext::validate` only re-derives the address from the stored bump; it
// never compares the passed account to `vault.vault_assets_account` the way
// `DepositContext`, `RedeemSharesContext` and `UpdateRewardContext` all do.
// EXPECTED TO FAIL.
// ---------------------------------------------------------------------------
#[rule]
fn rule_slash_assets_account_matches_recorded_pubkey() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    let amount: u64 = nondet();
    clog!(amount => "amount");
    clog!(Pk(accounts[1].key) => "vault_assets_account_key");
    clog!(pk_lo(accounts[1].key) => "vault_assets_account_key_lo64");
    clog!(Pk(&v0.vault_assets_account) => "vault.vault_assets_account");
    clog!(pk_lo(&v0.vault_assets_account) => "vault.vault_assets_account_lo64");
    clog!((*accounts[1].key == v0.vault_assets_account) as u64 => "key_matches_record");

    crate::processor::process_slash(&accounts[..6], amount).unwrap();

    cvlr_assert!(*accounts[1].key == v0.vault_assets_account);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 7: slash_cpi_uses_token_program_and_own_pda_seeds (routing half)
// ---------------------------------------------------------------------------
#[rule]
fn rule_slash_cpi_uses_token_program_and_own_pda_seeds() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);
    cpi_reset();

    let v0 = read_vault(&accounts[0]);
    let amount: u64 = nondet();
    clog!(amount => "amount");

    crate::processor::process_slash(&accounts[..6], amount).unwrap();

    let x = xfer_call();
    clog!(x.count => "cpi_count");
    clog!(Pk(&x.prog) => "cpi_target_program", Pk(&spl_token::ID) => "canonical_spl_token_id");
    clog!(Pk(accounts[5].key) => "spl_token_program_account_key");
    clog!(Pk(&x.src) => "transfer_source", Pk(accounts[1].key) => "vault_assets_account_key");
    clog!(Pk(&x.dst) => "transfer_destination", Pk(accounts[2].key) => "user_token_account_key");
    clog!(x.n_seeds => "n_signer_seeds");
    clog!(x.seed0_len => "seed0_len", x.seed1_len => "seed1_len", x.seed2_len => "seed2_len");
    clog!(x.seed0_tag => "seed0_tag", tag6(crate::constants::VAULT_ASSETS) => "expected_seed0_tag");
    clog!(Pk(&x.seed1_pk) => "seed1_pubkey", Pk(accounts[0].key) => "vault_info_key");
    clog!(x.seed2_bump as u64 => "seed2_bump");
    clog!(v0.vault_assets_account_bump as u64 => "vault.vault_assets_account_bump");

    // Exactly one transfer, to the canonical SPL Token program.
    cvlr_assert!(x.seen && x.count == 1);
    cvlr_assert!(x.prog == spl_token::ID);
    cvlr_assert!(*accounts[5].key == spl_token::ID);
    // Debiting the vault's own assets account, crediting the named recipient.
    cvlr_assert!(x.src == *accounts[1].key);
    cvlr_assert!(x.dst == *accounts[2].key);
    // Signed with exactly ["assets", vault_info.key, vault.vault_assets_account_bump]
    // — values taken from the validated vault state, never from instruction data.
    cvlr_assert!(x.n_seeds == 3);
    cvlr_assert!(x.seed0_len == crate::constants::VAULT_ASSETS.len() as u64);
    cvlr_assert!(x.seed0_tag == tag6(crate::constants::VAULT_ASSETS));
    cvlr_assert!(x.seed1_len == 32);
    cvlr_assert!(x.seed1_pk == *accounts[0].key);
    cvlr_assert!(x.seed2_len == 1);
    cvlr_assert!(x.seed2_bump == v0.vault_assets_account_bump);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 7 (instruction half): the transfer instruction the program builds
// targets the token program it was handed, debits its first account parameter,
// credits its second, and names the debited account itself as the transfer
// authority — never the reverse, and never a third party.
// ---------------------------------------------------------------------------
#[rule]
fn rule_slash_transfer_ix_builder() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());

    let amount: u64 = nondet();
    let from = *accounts[1].key;
    let to = *accounts[2].key;
    let prog = *accounts[5].key;
    clog!(amount => "amount");
    clog!(Pk(&from) => "param_vault_assets", Pk(&to) => "param_user_token_account");
    clog!(Pk(&prog) => "param_spl_token_program");

    let ix: Instruction = build_transfer_assets_from_vault_ix(amount, &from, &to, &prog).unwrap();
    clog!(ix.accounts.len() as u64 => "n_metas");
    clog!(Pk(&ix.program_id) => "ix_program_id");
    clog!(Pk(&ix.accounts[0].pubkey) => "meta0_source");
    clog!(Pk(&ix.accounts[1].pubkey) => "meta1_destination");
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

// ---------------------------------------------------------------------------
// Property 8: slash_assets_mint_not_bound_to_vault
//
// Stated as the property a correct program would have: a successful
// `process_slash` can only ever have been handed the vault's recorded asset
// mint in the `assets_mint` slot.  EXPECTED TO FAIL — `SlashContext::validate`
// omits the `require_eq!` against `vault.assets_mint` that `DepositContext` and
// `RedeemSharesContext` both perform.
// ---------------------------------------------------------------------------
#[rule]
fn rule_slash_assets_mint_matches_vault_record() {
    let accounts = Box::new(cvlr_deserialize_nondet_accounts());
    assume_vault_account_shape(&accounts[..]);

    let v0 = read_vault(&accounts[0]);
    let amount: u64 = nondet();
    clog!(amount => "amount");
    clog!(Pk(accounts[3].key) => "assets_mint_key", pk_lo(accounts[3].key) => "assets_mint_key_lo64");
    clog!(Pk(&v0.assets_mint) => "vault.assets_mint", pk_lo(&v0.assets_mint) => "vault.assets_mint_lo64");
    clog!((*accounts[3].key == v0.assets_mint) as u64 => "mint_matches_record");

    crate::processor::process_slash(&accounts[..6], amount).unwrap();

    cvlr_assert!(*accounts[3].key == v0.assets_mint);
    core::mem::forget(accounts);
}

// ---------------------------------------------------------------------------
// Property 9: slash_reachable_for_legitimate_admin
//
// Driven at the vault-accounting layer, which is where the only guard capable
// of rejecting a legitimately sized slash lives: the `checked_sub` and the
// post-decrement `check_invariant` inside `Vault::del_token`.  An
// instruction-level version of this rule is not statable here — `process_slash`
// reaches `vault_process_slash` only through `VaultAssetsAccount::check_vault`,
// whose `Pubkey::create_program_address` result the configuration leaves
// unconstrained, so no assumption a rule can state makes that comparison
// succeed.  See the commentary.
// ---------------------------------------------------------------------------
#[rule]
fn rule_slash_accounting_accepts_legitimate_amount() {
    let mut vault = Vault::default();
    vault.shares = nondet::<u64>().into();
    vault.assets = nondet::<u64>().into();

    let s0 = vault.num_shares();
    let a0 = vault.num_assets();
    assume_u64(s0);
    assume_u64(a0);

    let amount: u64 = nondet();
    assume_u64(amount);

    // A solvent vault and a slash inside the vault's free (unbacked) balance.
    cvlr_assume!(s0 <= a0);
    cvlr_assume!(amount > 0);
    cvlr_assume!(ni(amount) + ni(s0) <= ni(a0));
    clog!(s0 => "shares_before", a0 => "assets_before", amount => "amount");

    let res = crate::operations::vault_process_slash(&mut vault, amount);
    clog!(res.is_ok() as u64 => "accepted");
    clog!(vault.num_shares() => "shares_after", vault.num_assets() => "assets_after");

    cvlr_assert!(res.is_ok());
}
