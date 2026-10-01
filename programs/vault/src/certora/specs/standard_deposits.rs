//! CVLR rules for the **Standard Deposits** component of the vault program.
//!
//! The two handlers under test are `crate::processor::process_deposit` and
//! `crate::processor::process_deposit_exact`.
//!
//! Account order expected by `DepositContext::load`:
//!   0 vault_info, 1 vault_assets_account, 2 assets_mint, 3 shares_mint,
//!   4 user_assets_account, 5 authority (signer), 6 user_shares_account,
//!   7 spl_token_program.
//!
//! IMPORTANT: every handler/operation call below is made through a `macro_rules!`
//! expansion so that the call is a *direct* call to a named program function.  An
//! earlier draft routed the handlers through `fn` pointers; the prover runs with
//! `-solanaSkipCallRegInst true`, which skips indirect (register) calls, and several
//! rules then reported verdicts about an execution in which the handler never ran.

#![allow(dead_code)]
#![allow(static_mut_refs)]

use cvlr::mathint::NativeInt;
use cvlr::prelude::*;
use cvlr_solana::cvlr_deserialize_nondet_accounts;
use cvlr_solana::pubkey::Pk;
use cvlr_solana::token::{
    spl_mint_get_supply, spl_mint_to, spl_token_account_get_amount, spl_token_transfer,
};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use crate::state::Vault;

// ---------------------------------------------------------------------------
// Model of the SPL Token CPIs performed by the deposit handlers.
//
// The build summarizes the `invoke` family, so a real CPI moves nothing: with it,
// every property about token balances is unprovable (an increase in `vault.assets`
// against a frozen token balance).
//
// The two CPI wrapper functions of `crate::processor::spl_token_utils` are therefore
// replaced (`mock_fn` munge) by the stand-ins below.  Each one
//   * records what the handler asked the Token program to do (amount + the keys), and
//   * applies `cvlr-solana`'s own model of the Token program state transition, so the
//     token account `amount` / mint `supply` fields really move.
//
// `cvlr-solana`'s transfer model is a no-op when source and destination keys are
// equal, exactly as the real Token program is, which is what makes the self-transfer
// property meaningful.  What the model does *not* do: it ignores the `authority`
// argument entirely (no ownership or delegate check), it does not check that the
// destination belongs to the mint, it *assumes* rather than branches on the source
// holding enough, and the mint stand-in does not sign with `mint_signer_seeds`.
// ---------------------------------------------------------------------------

const ZERO_KEY: Pubkey = Pubkey::new_from_array([0u8; 32]);

static mut XFER_CALLS: u64 = 0;
static mut XFER_AMOUNT: u64 = 0;
static mut XFER_SRC: Pubkey = ZERO_KEY;
static mut XFER_DST: Pubkey = ZERO_KEY;
static mut XFER_AUTH: Pubkey = ZERO_KEY;
static mut MINT_CALLS: u64 = 0;
static mut MINT_AMOUNT: u64 = 0;
static mut MINT_MINT: Pubkey = ZERO_KEY;
static mut MINT_DST: Pubkey = ZERO_KEY;

fn cpi_log_reset() {
    unsafe {
        XFER_CALLS = 0;
        XFER_AMOUNT = 0;
        XFER_SRC = ZERO_KEY;
        XFER_DST = ZERO_KEY;
        XFER_AUTH = ZERO_KEY;
        MINT_CALLS = 0;
        MINT_AMOUNT = 0;
        MINT_MINT = ZERO_KEY;
        MINT_DST = ZERO_KEY;
    }
}

fn xfer_calls() -> u64 {
    unsafe { XFER_CALLS }
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
fn xfer_auth() -> Pubkey {
    unsafe { XFER_AUTH }
}
fn mint_calls() -> u64 {
    unsafe { MINT_CALLS }
}
fn mint_amount() -> u64 {
    unsafe { MINT_AMOUNT }
}
fn mint_mint() -> Pubkey {
    unsafe { MINT_MINT }
}
fn mint_dst() -> Pubkey {
    unsafe { MINT_DST }
}

/// Stand-in for `crate::processor::spl_token_utils::spl_transfer_assets_from_user`.
pub fn spl_transfer_assets_from_user<'a>(
    amount: u64,
    vault_assets: &AccountInfo<'a>,
    user_assets: &AccountInfo<'a>,
    _mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    _spl_token_program: &AccountInfo<'a>,
) -> ProgramResult {
    unsafe {
        XFER_CALLS += 1;
        XFER_AMOUNT = amount;
        XFER_SRC = *user_assets.key;
        XFER_DST = *vault_assets.key;
        XFER_AUTH = *authority.key;
    }
    spl_token_transfer(user_assets, vault_assets, authority, amount)
}

/// Stand-in for `crate::processor::spl_token_utils::spl_mint_shares`.
pub fn spl_mint_shares<'a>(
    amount: u64,
    user_shares_account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    _spl_token_program: &AccountInfo<'a>,
    _mint_signer_seeds: &[&[u8]],
) -> ProgramResult {
    unsafe {
        MINT_CALLS += 1;
        MINT_AMOUNT = amount;
        MINT_MINT = *mint.key;
        MINT_DST = *user_shares_account.key;
    }
    spl_mint_to(mint, user_shares_account, mint, amount)
}

// ---------------------------------------------------------------------------
// Harness plumbing
// ---------------------------------------------------------------------------

const VAULT_LEN: usize = core::mem::size_of::<Vault>();

/// Pin the size of every account the handlers parse.  A nondet account has a nondet
/// data length, which no deserializer can be analyzed against.
fn assume_well_shaped(a: &[AccountInfo]) {
    cvlr_assume!(a[0].data_len() == VAULT_LEN);
    cvlr_assume!(a[1].data_len() == spl_token::state::Account::LEN);
    cvlr_assume!(a[3].data_len() == spl_token::state::Mint::LEN);
    cvlr_assume!(a[4].data_len() == spl_token::state::Account::LEN);
    cvlr_assume!(a[6].data_len() == spl_token::state::Account::LEN);
}

/// Read a copy of the zero-copy `Vault` out of the account bytes, exactly the way
/// `VaultInfo::get` does.  The borrow is released before returning.
fn read_vault(info: &AccountInfo) -> Vault {
    let data = info.try_borrow_data().unwrap();
    *bytemuck::from_bytes::<Vault>(&data[0..VAULT_LEN])
}

fn ni(v: u64) -> NativeInt {
    NativeInt::from(v)
}

/// A nondeterministic vault *state*, for the rules that drive the accounting core
/// directly.  Only the numeric fields the core reads are made nondeterministic.
fn nondet_vault() -> Vault {
    let mut v = Vault::default();
    v.shares = nondet::<u64>().into();
    v.assets = nondet::<u64>().into();
    v.fee_bps = nondet::<u64>().into();
    v.fee_amount = nondet::<u64>().into();
    v
}

/// The share count the vault's current ratio prices `amount` assets at.
fn expected_shares(shares: u64, assets: u64, amount: u64) -> NativeInt {
    if shares == assets {
        ni(amount)
    } else {
        ni(amount).muldiv(ni(shares), ni(assets))
    }
}

macro_rules! deposit_env {
    ($accounts:ident, $a:ident) => {
        let $accounts = Box::new(cvlr_deserialize_nondet_accounts());
        let $a: &[AccountInfo] = &$accounts[..8];
        assume_well_shaped($a);
        cpi_log_reset();
    };
}

// ===========================================================================
// Property 1 — deposit_mints_shares_at_current_ratio
// ===========================================================================

#[rule]
pub fn rule_deposit_mints_shares_at_current_ratio() {
    deposit_env!(accounts, a);
    let v0 = read_vault(&a[0]);
    let (s0, a0) = (v0.num_shares(), v0.num_assets());
    let amount: u64 = nondet();
    clog!(s0, a0, amount);

    crate::processor::process_deposit(a, amount).unwrap();

    let v1 = read_vault(&a[0]);
    let (s1, a1) = (v1.num_shares(), v1.num_assets());
    let expected = expected_shares(s0, a0, amount);
    clog!(s1, a1);
    clog!(expected);

    cvlr_assert!(ni(s1) == ni(s0) + expected);
    cvlr_assert!(ni(a1) == ni(a0) + ni(amount));
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 2 — deposit_never_dilutes_existing_shareholders
//   assets_after * shares_before >= assets_before * shares_after
// ===========================================================================

macro_rules! body_no_dilution {
    ($h:path) => {{
        deposit_env!(accounts, a);
        let v0 = read_vault(&a[0]);
        let (s0, a0) = (v0.num_shares(), v0.num_assets());
        let amount: u64 = nondet();
        clog!(s0, a0, amount);

        $h(a, amount).unwrap();

        let v1 = read_vault(&a[0]);
        let (s1, a1) = (v1.num_shares(), v1.num_assets());
        clog!(s1, a1);

        cvlr_assert!(ni(a1) * ni(s0) >= ni(a0) * ni(s1));
        core::mem::forget(accounts);
    }};
}

#[rule]
pub fn rule_no_dilution_deposit() {
    body_no_dilution!(crate::processor::process_deposit);
}

#[rule]
pub fn rule_no_dilution_deposit_exact() {
    body_no_dilution!(crate::processor::process_deposit_exact);
}

// ===========================================================================
// Property 3 — deposits_preserve_vault_solvency
// ===========================================================================

macro_rules! body_solvency {
    ($h:path) => {{
        deposit_env!(accounts, a);
        let v0 = read_vault(&a[0]);
        let (s0, a0) = (v0.num_shares(), v0.num_assets());
        cvlr_assume!(s0 <= a0);
        let amount: u64 = nondet();
        clog!(s0, a0, amount);

        $h(a, amount).unwrap();

        let v1 = read_vault(&a[0]);
        let (s1, a1) = (v1.num_shares(), v1.num_assets());
        clog!(s1, a1);

        cvlr_assert!(s1 <= a1);
        core::mem::forget(accounts);
    }};
}

#[rule]
pub fn rule_solvency_deposit() {
    body_solvency!(crate::processor::process_deposit);
}

#[rule]
pub fn rule_solvency_deposit_exact() {
    body_solvency!(crate::processor::process_deposit_exact);
}

// ===========================================================================
// Property 4 — deposit_must_reject_self_transfer
//
// Three assumptions beyond the property's own antecedent, each of which only
// *narrows* the executions considered and so cannot manufacture a violation:
// a signing authority and a positive `amount` (an attacker calling this really
// does sign and really does pass a positive amount), and `vault.shares ==
// vault.assets`, which keeps the handler's share pricing on its 1:1 branch.  The
// rule has to *find* a successful execution, and with the `mul_div_floor` ratio
// branch and the non-signer error paths both live that search did not terminate
// within the solver budget.  The defect is not branch-specific; the exhibited
// witness is.
// ===========================================================================

#[rule]
pub fn rule_deposit_must_reject_self_transfer() {
    deposit_env!(accounts, a);
    // the depositor's asset account *is* the vault's asset account
    cvlr_assume!(a[1].key == a[4].key);
    cvlr_assume!(a[5].is_signer);
    let v0 = read_vault(&a[0]);
    cvlr_assume!(v0.num_shares() == v0.num_assets());
    let bal0 = spl_token_account_get_amount(&a[1]);
    let amount: u64 = nondet();
    cvlr_assume!(amount > 0);
    clog!(v0.num_shares(), v0.num_assets(), amount, bal0);

    let r = crate::processor::process_deposit(a, amount);

    let v1 = read_vault(&a[0]);
    let bal1 = spl_token_account_get_amount(&a[1]);
    clog!(v1.num_shares(), v1.num_assets(), bal1);
    clog!(r.is_err());
    cvlr_assert!(r.is_err());
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 5 — deposit_cpi_amounts_match_recorded_state_deltas
//
// Part (a): observed through `cvlr-solana`'s own token model — the amounts that
// actually moved between the token accounts named by the property.  Restricted to
// `user_assets_account != vault_assets_account`; the unrestricted form of the
// property — the amount *requested* in each CPI equals the recorded delta — is what
// part (b) below establishes, with no exclusion.
// ===========================================================================

macro_rules! body_cpi_token_effects {
    ($h:path) => {{
        deposit_env!(accounts, a);
        // Not the self-transfer case (that is Property 4's subject, and the Token
        // program models a self transfer as moving nothing).
        cvlr_assume!(a[1].key != a[4].key);
        let v0 = read_vault(&a[0]);
        let (s0, a0) = (v0.num_shares(), v0.num_assets());
        let vb0 = spl_token_account_get_amount(&a[1]);
        let ub0 = spl_token_account_get_amount(&a[4]);
        let sup0 = spl_mint_get_supply(&a[3]);
        let usb0 = spl_token_account_get_amount(&a[6]);
        let amount: u64 = nondet();
        clog!(s0, a0, amount);
        clog!(vb0, ub0, sup0, usb0);

        $h(a, amount).unwrap();

        let v1 = read_vault(&a[0]);
        let (s1, a1) = (v1.num_shares(), v1.num_assets());
        let vb1 = spl_token_account_get_amount(&a[1]);
        let ub1 = spl_token_account_get_amount(&a[4]);
        let sup1 = spl_mint_get_supply(&a[3]);
        let usb1 = spl_token_account_get_amount(&a[6]);
        clog!(s1, a1);
        clog!(vb1, ub1, sup1, usb1);

        // the asset transfer: user_assets -> vault_assets, of exactly the increase
        // in vault.assets
        cvlr_assert!(ni(vb0) + ni(a1) == ni(vb1) + ni(a0));
        cvlr_assert!(ni(ub0) + ni(a0) == ni(ub1) + ni(a1));
        // the share mint: shares_mint -> user_shares_account, of exactly the
        // increase in vault.shares
        cvlr_assert!(ni(sup0) + ni(s1) == ni(sup1) + ni(s0));
        cvlr_assert!(ni(usb0) + ni(s1) == ni(usb1) + ni(s0));
        core::mem::forget(accounts);
    }};
}

#[rule]
pub fn rule_cpi_token_effects_deposit() {
    body_cpi_token_effects!(crate::processor::process_deposit);
}

#[rule]
pub fn rule_cpi_token_effects_deposit_exact() {
    body_cpi_token_effects!(crate::processor::process_deposit_exact);
}

// Part (b): observed through the recorded CPI arguments — exactly one transfer and
// one mint_to, with the amounts and *all* the account keys the property names,
// including the signing `authority`, which the token model itself ignores.  No
// account-aliasing exclusion here, so this half of Property 5 is unconditional.
macro_rules! body_cpi_arguments {
    ($h:path) => {{
        deposit_env!(accounts, a);
        let v0 = read_vault(&a[0]);
        let (s0, a0) = (v0.num_shares(), v0.num_assets());
        let amount: u64 = nondet();
        clog!(s0, a0, amount);

        $h(a, amount).unwrap();

        let v1 = read_vault(&a[0]);
        let (s1, a1) = (v1.num_shares(), v1.num_assets());
        clog!(s1, a1);
        clog!(xfer_calls(), xfer_amount(), mint_calls(), mint_amount());
        clog!(Pk(&xfer_src()) => "xfer src", Pk(&xfer_dst()) => "xfer dst");
        clog!(Pk(&xfer_auth()) => "xfer authority");
        clog!(Pk(&mint_mint()) => "mint", Pk(&mint_dst()) => "mint dst");

        cvlr_assert!(xfer_calls() == 1);
        cvlr_assert!(mint_calls() == 1);
        cvlr_assert!(ni(a0) + ni(xfer_amount()) == ni(a1));
        cvlr_assert!(ni(s0) + ni(mint_amount()) == ni(s1));
        cvlr_assert!(xfer_src() == *a[4].key);
        cvlr_assert!(xfer_dst() == *a[1].key);
        cvlr_assert!(xfer_auth() == *a[5].key);
        cvlr_assert!(mint_mint() == v0.shares_mint);
        cvlr_assert!(mint_dst() == *a[6].key);
        core::mem::forget(accounts);
    }};
}

#[rule]
pub fn rule_cpi_arguments_deposit() {
    body_cpi_arguments!(crate::processor::process_deposit);
}

#[rule]
pub fn rule_cpi_arguments_deposit_exact() {
    body_cpi_arguments!(crate::processor::process_deposit_exact);
}

// ===========================================================================
// Property 6 — recorded_assets_remain_backed_by_token_balance
//
// Two rules per handler-pair, because the invariant genuinely splits on the
// self-transfer input:
//   * the restricted rules below hold for `user_assets_account !=
//     vault_assets_account`.  For `process_deposit_exact` the restriction is
//     redundant — that handler enforces it itself with a live `require_ne!` — so
//     its verdict is unconditional.
//   * `rule_backed_by_token_balance_self_transfer_deposit` covers the excluded
//     input for `process_deposit`, and is expected to fail: the Property 4 defect
//     lets the handler credit `vault.assets` for a transfer that moved nothing.
// ===========================================================================

macro_rules! body_backed_by_balance {
    ($h:path) => {{
        deposit_env!(accounts, a);
        cvlr_assume!(a[1].key != a[4].key);
        let v0 = read_vault(&a[0]);
        let bal0 = spl_token_account_get_amount(&a[1]);
        cvlr_assume!(ni(v0.num_assets()) + ni(v0.fee_amount()) <= ni(bal0));
        let amount: u64 = nondet();
        clog!(v0.num_assets(), v0.fee_amount(), bal0, amount);

        $h(a, amount).unwrap();

        let v1 = read_vault(&a[0]);
        let bal1 = spl_token_account_get_amount(&a[1]);
        clog!(v1.num_assets(), v1.fee_amount(), bal1);

        cvlr_assert!(ni(v1.num_assets()) + ni(v1.fee_amount()) <= ni(bal1));
        core::mem::forget(accounts);
    }};
}

#[rule]
pub fn rule_backed_by_token_balance_deposit() {
    body_backed_by_balance!(crate::processor::process_deposit);
}

#[rule]
pub fn rule_backed_by_token_balance_deposit_exact() {
    body_backed_by_balance!(crate::processor::process_deposit_exact);
}

/// Property 6 on the input the restricted rule excludes.  Identical to
/// `rule_backed_by_token_balance_deposit` except that the two asset token accounts
/// are the *same* account.  Expected to fail.
#[rule]
pub fn rule_backed_by_token_balance_self_transfer_deposit() {
    deposit_env!(accounts, a);
    cvlr_assume!(a[1].key == a[4].key);
    let v0 = read_vault(&a[0]);
    let rec0 = v0.num_assets();
    let fee0 = v0.fee_amount();
    let bal0 = spl_token_account_get_amount(&a[1]);
    cvlr_assume!(ni(rec0) + ni(fee0) <= ni(bal0));
    let amount: u64 = nondet();
    clog!(rec0, fee0, bal0, amount);

    crate::processor::process_deposit(a, amount).unwrap();

    let v1 = read_vault(&a[0]);
    let rec1 = v1.num_assets();
    let fee1 = v1.fee_amount();
    let bal1 = spl_token_account_get_amount(&a[1]);
    clog!(rec1, fee1, bal1);

    cvlr_assert!(ni(rec1) + ni(fee1) <= ni(bal1));
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 7 — exact_deposit_never_takes_more_than_requested
// ===========================================================================

#[rule]
pub fn rule_exact_deposit_never_takes_more_than_requested() {
    deposit_env!(accounts, a);
    let v0 = read_vault(&a[0]);
    let (s0, a0) = (v0.num_shares(), v0.num_assets());
    let ub0 = spl_token_account_get_amount(&a[4]);
    let amount: u64 = nondet();
    clog!(s0, a0, amount, ub0);

    crate::processor::process_deposit_exact(a, amount).unwrap();

    let v1 = read_vault(&a[0]);
    let a1 = v1.num_assets();
    let ub1 = spl_token_account_get_amount(&a[4]);
    clog!(a1, ub1);

    // the vault credits itself at most `amount` ...
    cvlr_assert!(ni(a1) <= ni(a0) + ni(amount));
    // ... and at most `amount` actually left the depositor's token account
    cvlr_assert!(ni(ub0) <= ni(ub1) + ni(amount));
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 8 — deposit_rejects_zero_share_or_zero_asset_move
// ===========================================================================

macro_rules! body_nonzero_moves {
    ($h:path) => {{
        deposit_env!(accounts, a);
        let v0 = read_vault(&a[0]);
        let (s0, a0) = (v0.num_shares(), v0.num_assets());
        let amount: u64 = nondet();
        clog!(s0, a0, amount);

        $h(a, amount).unwrap();

        let v1 = read_vault(&a[0]);
        let (s1, a1) = (v1.num_shares(), v1.num_assets());
        clog!(s1, a1);

        cvlr_assert!(s1 > s0);
        cvlr_assert!(a1 > a0);
        core::mem::forget(accounts);
    }};
}

#[rule]
pub fn rule_nonzero_moves_deposit() {
    body_nonzero_moves!(crate::processor::process_deposit);
}

#[rule]
pub fn rule_nonzero_moves_deposit_exact() {
    body_nonzero_moves!(crate::processor::process_deposit_exact);
}

// ===========================================================================
// Property 9 — deposit_leaves_vault_configuration_unchanged
// ===========================================================================

macro_rules! body_config_unchanged {
    ($h:path) => {{
        deposit_env!(accounts, a);
        let v0 = read_vault(&a[0]);
        let amount: u64 = nondet();
        clog!(v0.num_shares(), v0.num_assets(), amount);

        $h(a, amount).unwrap();

        let v1 = read_vault(&a[0]);
        clog!(u64::from(v1.fee_bps), v1.fee_amount());
        clog!(v1.vault_shares_mint_bump as u64, v1.vault_assets_account_bump as u64);
        clog!(Pk(&v1.admin) => "admin after", Pk(&v1.shares_mint) => "shares_mint after");

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
    }};
}

#[rule]
pub fn rule_config_unchanged_deposit() {
    body_config_unchanged!(crate::processor::process_deposit);
}

#[rule]
pub fn rule_config_unchanged_deposit_exact() {
    body_config_unchanged!(crate::processor::process_deposit_exact);
}

// ===========================================================================
// Property 10 — deposit_must_reject_foreign_owned_vault_account
//
// Stated against `DepositContext::load`, the *shared* account-validation entry
// point that is the first statement of both `process_deposit` and
// `process_deposit_exact` and whose result both `?`-propagate.  `VaultInfo::
// try_from` (reached from `load`) carries the comments `// owned by vault program`
// and `// has discriminant`, and neither check was written; nothing later in either
// handler looks at `vault_info.owner` either.  So acceptance here is acceptance by
// the handlers — but note that is a reading of the source, not something the rule
// exercises, and the rule shows validation accepting a foreign-owned account rather
// than a completed exploit transaction.
//
// The handler-level form of this rule did not terminate within the solver budget:
// it keeps the error path live *and* has to find a fully successful execution
// through the ratio arithmetic and both token CPIs.
// ===========================================================================

#[rule]
pub fn rule_reject_foreign_owned_vault() {
    deposit_env!(accounts, a);
    // the "vault" account is owned by somebody other than this program
    cvlr_assume!(*a[0].owner != crate::ID);
    let v0 = read_vault(&a[0]);
    clog!(v0.num_shares(), v0.num_assets());
    clog!(Pk(a[0].owner) => "vault_info owner", Pk(&crate::ID) => "vault program id");

    let r = crate::loaders::DepositContext::load(a);
    let accepted = r.is_ok();
    core::mem::forget(r);
    clog!(accepted);

    cvlr_assert!(!accepted);
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 11 — deposit_requires_authority_signature
// ===========================================================================

macro_rules! body_requires_signature {
    ($h:path) => {{
        deposit_env!(accounts, a);
        cvlr_assume!(!a[5].is_signer);
        let amount: u64 = nondet();
        clog!(amount);

        let r = $h(a, amount);
        clog!(r.is_err());
        cvlr_assert!(r.is_err());
        core::mem::forget(accounts);
    }};
}

#[rule]
pub fn rule_requires_authority_signature_deposit() {
    body_requires_signature!(crate::processor::process_deposit);
}

#[rule]
pub fn rule_requires_authority_signature_deposit_exact() {
    body_requires_signature!(crate::processor::process_deposit_exact);
}

// ===========================================================================
// Property 12 — standard_deposit_must_not_bypass_configured_fee
//
// Stated against the deposit accounting core, which is where the fee guard used to
// live (`vault_deposit_assets` still carries it, commented out) and the only place
// on either handler's path where `vault.fee_bps` could be consulted.  The vault
// state is otherwise constrained to one on which the deposit would plainly succeed,
// so a violation exhibits a *completed* fee-free deposit on a fee-configured vault
// — at the accounting layer; account validation and the token CPIs are outside it.
//
// The handler-level form of this rule did not terminate within the solver budget.
// ===========================================================================

macro_rules! body_no_fee_bypass {
    ($op:path) => {{
        let mut v = nondet_vault();
        let (s0, a0) = (v.num_shares(), v.num_assets());
        let fee_bps = u64::from(v.fee_bps);
        // a validly configured, non-zero fee
        cvlr_assume!(fee_bps != 0);
        cvlr_assume!(fee_bps <= 10_000);
        // ... and an otherwise perfectly ordinary, solvent vault and deposit
        let amount: u64 = nondet();
        cvlr_assume!(s0 > 0);
        cvlr_assume!(a0 > 0);
        cvlr_assume!(s0 <= a0);
        cvlr_assume!(amount > 0);
        let expected = expected_shares(s0, a0, amount);
        cvlr_assume!(expected >= 1u64);
        cvlr_assume!(ni(s0) + expected <= NativeInt::u64_max());
        cvlr_assume!(ni(a0) + ni(amount) <= NativeInt::u64_max());

        let r = $op(&mut v, amount);
        clog!(fee_bps, s0, a0, amount);
        clog!(r.is_err());
        // a fee-free deposit must not succeed on a vault that charges a fee
        cvlr_assert!(r.is_err());
    }};
}

#[rule]
pub fn rule_no_fee_bypass_deposit() {
    body_no_fee_bypass!(crate::operations::vault_deposit_assets);
}

#[rule]
pub fn rule_no_fee_bypass_deposit_exact() {
    body_no_fee_bypass!(crate::operations::vault_deposit_assets_exact);
}

// ===========================================================================
// Property 13 — legitimate_deposit_is_reachable
//
// Two complementary rules:
//  * a *universal* acceptance claim against the deposit accounting core — for
//    every well-formed, solvent vault state and every `amount` large enough to be
//    worth a share, `vault_deposit_assets` succeeds.  This is the half that must
//    hold for *all* such inputs, and it is stated where the guards actually live;
//    account validation and the CPIs are outside it.
//  * an existential witness at the full handler level, showing that a call with
//    distinct user/vault asset accounts and a signing authority really can get all
//    the way through `DepositContext::load`'s validation and mint shares.  (A
//    universal acceptance claim is not statable at handler level here: the build
//    summarizes `Pubkey::create_program_address`, so the two PDA re-derivation
//    checks compare against an unconstrained address and may refuse any account —
//    and conversely the witness satisfies them against an unconstrained value.)
// ===========================================================================

#[rule]
pub fn rule_legitimate_deposit_is_reachable() {
    let mut v = nondet_vault();
    let (s0, a0) = (v.num_shares(), v.num_assets());
    let amount: u64 = nondet();
    cvlr_assume!(s0 > 0);
    cvlr_assume!(a0 > 0);
    cvlr_assume!(amount > 0);
    cvlr_assume!(s0 <= a0); // a solvent vault
    let expected = expected_shares(s0, a0, amount);
    cvlr_assume!(expected >= 1u64); // amount large enough to be worth a share
    cvlr_assume!(ni(s0) + expected <= NativeInt::u64_max());
    cvlr_assume!(ni(a0) + ni(amount) <= NativeInt::u64_max());

    let r = crate::operations::vault_deposit_assets(&mut v, amount);
    clog!(s0, a0, amount);
    clog!(expected);
    clog!(r.is_ok());
    cvlr_assert!(r.is_ok());
}

#[rule]
pub fn rule_legitimate_deposit_handler_path_live() {
    deposit_env!(accounts, a);
    cvlr_assume!(a[1].key != a[4].key);
    cvlr_assume!(a[5].is_signer);
    let v0 = read_vault(&a[0]);
    // a vault at a 1:1 ratio; keeps the witness search off the `mul_div_floor`
    // branch, which did not terminate within the solver budget
    cvlr_assume!(v0.num_shares() == v0.num_assets());
    let amount: u64 = nondet();
    cvlr_assume!(amount > 0);
    clog!(v0.num_shares(), v0.num_assets(), amount);

    let r = crate::processor::process_deposit(a, amount);
    let v1 = read_vault(&a[0]);
    clog!(v1.num_shares(), v1.num_assets());
    cvlr_satisfy!(r.is_ok() && v1.num_shares() > v0.num_shares());
    core::mem::forget(accounts);
}

// ===========================================================================
// Property 14 — deposit_dead_when_shares_zero_but_assets_positive
//
// Stated against the accounting core, which is where the share count is computed
// and where the zero-share rejection lives.  Account validation, the self-transfer
// guard and the token CPIs are outside these rules.
// ===========================================================================

macro_rules! body_dead_when_shares_zero {
    ($op:path) => {{
        let mut v = nondet_vault();
        let (s0, a0) = (v.num_shares(), v.num_assets());
        cvlr_assume!(s0 == 0);
        cvlr_assume!(a0 > 0);
        let amount: u64 = nondet();
        cvlr_assume!(amount > 0);
        cvlr_assume!(ni(a0) + ni(amount) <= NativeInt::u64_max());

        let r = $op(&mut v, amount);
        clog!(s0, a0, amount);
        clog!(r.is_ok());
        // A vault holding assets but no shares should still be able to take a
        // deposit (a bootstrap ratio); if no `amount` can succeed, deposits are
        // bricked and the residual assets are unredeemable.
        cvlr_assert!(r.is_ok());
    }};
}

#[rule]
pub fn rule_deposit_not_dead_when_shares_zero() {
    body_dead_when_shares_zero!(crate::operations::vault_deposit_assets);
}

#[rule]
pub fn rule_deposit_exact_not_dead_when_shares_zero() {
    body_dead_when_shares_zero!(crate::operations::vault_deposit_assets_exact);
}

// ===========================================================================
// Property 15 — deposit_never_overcharges_depositor_for_shares
// ===========================================================================

macro_rules! body_no_overcharge {
    ($h:path) => {{
        deposit_env!(accounts, a);
        let v0 = read_vault(&a[0]);
        let (s0, a0) = (v0.num_shares(), v0.num_assets());
        let amount: u64 = nondet();
        clog!(s0, a0, amount);

        $h(a, amount).unwrap();

        let v1 = read_vault(&a[0]);
        let (s1, a1) = (v1.num_shares(), v1.num_assets());
        let d = ni(a1) - ni(a0); // assets taken from the depositor
        let s = ni(s1) - ni(s0); // shares minted
        clog!(s1, a1);
        clog!(d, s);

        if s0 == a0 {
            cvlr_assert!(d <= s);
        } else {
            // the pre-state value of the minted shares, rounded up
            let fair = s.muldiv_ceil(ni(a0), ni(s0));
            clog!(fair);
            cvlr_assert!(d <= fair);
        }
        core::mem::forget(accounts);
    }};
}

#[rule]
pub fn rule_no_overcharge_deposit() {
    body_no_overcharge!(crate::processor::process_deposit);
}

#[rule]
pub fn rule_no_overcharge_deposit_exact() {
    body_no_overcharge!(crate::processor::process_deposit_exact);
}

// ===========================================================================
// Property 16 — vault_shares_equals_shares_mint_supply
// ===========================================================================

macro_rules! body_shares_match_supply {
    ($h:path) => {{
        deposit_env!(accounts, a);
        let v0 = read_vault(&a[0]);
        let sup0 = spl_mint_get_supply(&a[3]);
        cvlr_assume!(v0.num_shares() == sup0);
        let amount: u64 = nondet();
        clog!(v0.num_shares(), sup0, amount);

        $h(a, amount).unwrap();

        let v1 = read_vault(&a[0]);
        let sup1 = spl_mint_get_supply(&a[3]);
        clog!(v1.num_shares(), sup1);

        cvlr_assert!(v1.num_shares() == sup1);
        core::mem::forget(accounts);
    }};
}

#[rule]
pub fn rule_shares_match_mint_supply_deposit() {
    body_shares_match_supply!(crate::processor::process_deposit);
}

#[rule]
pub fn rule_shares_match_mint_supply_deposit_exact() {
    body_shares_match_supply!(crate::processor::process_deposit_exact);
}

// ===========================================================================
// Property 17 — deposit_then_redeem_is_not_profitable
//
// Two consecutive transactions on one vault.  Stated against the program's own
// accounting core, because the redeem handler takes the same accounts in a
// different order and nothing in the property is about account validation.
//
// The pre-state is bounded by the vault's own guarantees: `shares <= assets` is
// `Vault::is_solvent`, which `add_token`/`del_token` maintain, and the two `u64`
// ceilings restate the `checked_add`s in `mint_shares`/`add_token`.  Without them
// the solver explores post-states above `u64::MAX` that the program cannot hold
// and reports an imprecise division as a counterexample.  Those bounds are
// *assumed* here, not proved at this point.
// ===========================================================================

macro_rules! body_deposit_then_redeem {
    ($op:path) => {{
        let mut v = nondet_vault();
        let (s0, a0) = (v.num_shares(), v.num_assets());
        let amount: u64 = nondet();
        cvlr_assume!(s0 > 0);
        cvlr_assume!(a0 > 0);
        cvlr_assume!(s0 <= a0);
        cvlr_assume!(ni(s0) + ni(amount) <= NativeInt::u64_max());
        cvlr_assume!(ni(a0) + ni(amount) <= NativeInt::u64_max());

        let dep = $op(&mut v, amount).unwrap();
        let (s1, a1) = (v.num_shares(), v.num_assets());
        let red = crate::operations::vault_redeem_shares(&mut v, dep.shares_to_user).unwrap();

        clog!(s0, a0, amount);
        clog!(s1, a1);
        clog!(dep.shares_to_user, dep.assets_to_vault, red.assets_to_user);

        cvlr_assert!(red.assets_to_user <= dep.assets_to_vault);
    }};
}

#[rule]
pub fn rule_deposit_then_redeem_not_profitable() {
    body_deposit_then_redeem!(crate::operations::vault_deposit_assets);
}

#[rule]
pub fn rule_deposit_exact_then_redeem_not_profitable() {
    body_deposit_then_redeem!(crate::operations::vault_deposit_assets_exact);
}

// ===========================================================================
// Property 18 — deposit_prices_shares_against_stale_asset_balance
//
// `shares_before > 0` is part of the property's own premise: the claim is that
// *existing shareholders* are diluted, and a vault with no outstanding shares has
// none.  Without it the solver settles on the degenerate `shares_before == 0`
// witness, for which the inequality is unsatisfiable for arithmetic rather than
// economic reasons.  The self-transfer input is excluded so that the rule reports
// its own property rather than Property 4's defect; both exclusions only narrow a
// rule that is expected to fail.
// ===========================================================================

macro_rules! body_stale_balance_pricing {
    ($h:path) => {{
        deposit_env!(accounts, a);
        cvlr_assume!(a[1].key != a[4].key);
        let v0 = read_vault(&a[0]);
        let s0 = v0.num_shares();
        let rec0 = v0.num_assets();
        let fee0 = v0.fee_amount();
        // there really are pre-existing shareholders to dilute ...
        cvlr_assume!(s0 > 0);
        // ... in a solvent vault, so the witness is a state the program can be in
        cvlr_assume!(s0 <= rec0);
        let bal0 = spl_token_account_get_amount(&a[1]);
        // ... and an unrecognized donation / accrued reward sits in the vault's
        // token account
        cvlr_assume!(ni(rec0) + ni(fee0) < ni(bal0));
        let surplus = bal0 - rec0 - fee0;
        let amount: u64 = nondet();
        clog!(s0, rec0, fee0, bal0);
        clog!(surplus, amount);

        $h(a, amount).unwrap();

        let v1 = read_vault(&a[0]);
        let s1 = v1.num_shares();
        let bal1 = spl_token_account_get_amount(&a[1]);
        clog!(s1, v1.num_assets(), bal1);

        cvlr_assert!(ni(bal1) * ni(s0) >= ni(bal0) * ni(s1));
        core::mem::forget(accounts);
    }};
}

#[rule]
pub fn rule_stale_balance_pricing_deposit() {
    body_stale_balance_pricing!(crate::processor::process_deposit);
}

#[rule]
pub fn rule_stale_balance_pricing_deposit_exact() {
    body_stale_balance_pricing!(crate::processor::process_deposit_exact);
}
