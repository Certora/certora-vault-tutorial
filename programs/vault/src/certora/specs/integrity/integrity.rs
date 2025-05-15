use crate::processor::{process_deposit, process_redeem_shares, process_slash};
use crate::state::Vault;
use cvlr::mathint::NativeInt;
use cvlr::prelude::*;
use cvlr_solana::cvlr_deserialize_nondet_accounts;
use solana_program::account_info::next_account_info;
use std::mem::size_of;

macro_rules! get_vault_total_assets {
    ($account_info:expr) => {{
        let data = $account_info.try_borrow_data().unwrap();
        let vault = bytemuck::from_bytes::<Vault>(&data[0..size_of::<Vault>()]);
        let total_assets = vault.num_assets();
        cvlr_assume!(cvlr::mathint::is_u64(total_assets));
        total_assets
    }};
}

macro_rules! get_vault_total_shares {
    ($account_info:expr) => {{
        let data = $account_info.try_borrow_data().unwrap();
        let vault = bytemuck::from_bytes::<Vault>(&data[0..size_of::<Vault>()]);
        let total_shares = vault.num_shares();
        cvlr_assume!(cvlr::mathint::is_u64(total_shares));
        total_shares
    }};
}

#[rule]
pub fn rule_deposit_must_increase_assets() {
    let accounts = cvlr_deserialize_nondet_accounts();
    let iter = &mut accounts.iter();
    let vault_info = next_account_info(iter).unwrap();
    let total_assets_pre = get_vault_total_assets!(vault_info);
    let amount = nondet();

    process_deposit(&accounts, amount).unwrap();

    let total_assets_post = get_vault_total_assets!(vault_info);
    clog!(amount, total_assets_pre, total_assets_post);
    cvlr_assert_ge!(total_assets_post, total_assets_pre);
}

#[rule]
pub fn rule_deposit_must_increase_shares() {
    let accounts = cvlr_deserialize_nondet_accounts();
    let iter = &mut accounts.iter();
    let vault_info = next_account_info(iter).unwrap();
    let total_shares_pre = get_vault_total_shares!(vault_info);
    let amount = nondet();

    process_deposit(&accounts, amount).unwrap();

    let total_shares_post = get_vault_total_shares!(vault_info);
    clog!(amount, total_shares_pre, total_shares_post);
    cvlr_assert_ge!(total_shares_post, total_shares_pre);
}

#[rule]
pub fn rule_deposit_assets_and_shares_monotonicity() {
    let accounts = cvlr_deserialize_nondet_accounts();
    let iter = &mut accounts.iter();
    let vault_info = next_account_info(iter).unwrap();
    let total_assets_pre = get_vault_total_assets!(vault_info);
    let total_shares_pre = get_vault_total_shares!(vault_info);
    let amount = nondet();

    process_deposit(&accounts, amount).unwrap();

    let total_assets_post = get_vault_total_assets!(vault_info);
    let total_shares_post = get_vault_total_shares!(vault_info);
    clog!(
        amount,
        total_shares_pre,
        total_shares_post,
        total_assets_pre,
        total_assets_post
    );
    if total_assets_pre <= total_assets_post {
        cvlr_assert_le!(total_shares_pre, total_shares_post);
    }
}

#[rule]
pub fn rule_redeem_assets_and_shares_monotonicity() {
    let accounts = cvlr_deserialize_nondet_accounts();
    let iter = &mut accounts.iter();
    let vault_info = next_account_info(iter).unwrap();
    let total_assets_pre = get_vault_total_assets!(vault_info);
    let total_shares_pre = get_vault_total_shares!(vault_info);
    let amount = nondet();

    // vault must be solvent
    cvlr_assume!(total_shares_pre <= total_assets_pre);

    process_redeem_shares(&accounts, amount).unwrap();

    let total_assets_post = get_vault_total_assets!(vault_info);
    let total_shares_post = get_vault_total_shares!(vault_info);

    clog!(
        amount,
        total_shares_pre,
        total_shares_post,
        total_assets_pre,
        total_assets_post
    );
    if total_assets_pre <= total_assets_post {
        cvlr_assert_le!(total_shares_pre, total_shares_post);
    }
}

#[rule]
pub fn rule_slash_no_dilution() {
    let accounts = cvlr_deserialize_nondet_accounts();
    let iter = &mut accounts.iter();
    let vault_info = next_account_info(iter).unwrap();

    let total_assets_pre: NativeInt = get_vault_total_assets!(vault_info).into();
    let total_shares_pre: NativeInt = get_vault_total_shares!(vault_info).into();

    let amount = nondet();
    process_slash(&accounts, amount).unwrap();

    let total_assets_post: NativeInt = get_vault_total_assets!(vault_info).into();
    let total_shares_post: NativeInt = get_vault_total_shares!(vault_info).into();

    clog!(
        total_assets_pre,
        total_shares_pre,
        total_assets_post,
        total_shares_post
    );

    // the ratio total_assets / total_shares cannot decrease
    cvlr_assert_le!(
        total_assets_pre * total_shares_post,
        total_shares_pre * total_assets_post
    );
}
