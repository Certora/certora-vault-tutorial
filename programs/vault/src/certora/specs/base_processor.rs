use cvlr::log::CvlrLog;
use cvlr::clog;
use cvlr::nondet;
use crate::processor::{process_deposit, process_redeem_shares, process_update_reward};
use solana_program::account_info::{AccountInfo, next_account_info};

pub trait CvlrProp: CvlrLog {
    fn new(vault_info: &AccountInfo,
           vault_assets_account: &AccountInfo,
           assets_mint: Option<&AccountInfo>,
           shares_mint: Option<&AccountInfo>,
           user_assets_account: Option<&AccountInfo>,
           authority: Option<&AccountInfo>,
           user_shares_account: Option<&AccountInfo>) -> Self;
    fn assume_pre(&self);
    fn check_post(&self, old: &Self);
}

#[inline(always)]
pub fn base_process_deposit<C: CvlrProp>(accounts: &[AccountInfo]) {

    let iter = &mut accounts.iter();
    let vault_info = next_account_info(iter).unwrap();
    let vault_assets_account= next_account_info(iter).unwrap();
    let assets_mint = next_account_info(iter).unwrap();
    let shares_mint = next_account_info(iter).unwrap();
    let user_assets_account = next_account_info(iter).unwrap();
    let authority = next_account_info(iter).unwrap();
    let user_shares_account = next_account_info(iter).unwrap();
 
    let pre = C::new(vault_info, 
                        vault_assets_account, 
                        Some(assets_mint), 
                        Some(shares_mint), 
                        Some(user_assets_account), 
                        Some(authority), 
                        Some(user_shares_account));
    pre.assume_pre();

    let amount = nondet();
    process_deposit(accounts, amount).unwrap();

    let post = C::new(vault_info, 
        vault_assets_account, 
        Some(assets_mint), 
        Some(shares_mint), 
        Some(user_assets_account), 
        Some(authority), 
        Some(user_shares_account));

    clog!(pre, post);
    post.check_post(&pre);
}


#[inline(always)]
pub fn base_process_redeem_shares<C: CvlrProp>(accounts: &[AccountInfo]) {
    let iter = &mut accounts.iter();
    let vault_info = next_account_info(iter).unwrap();
    let vault_assets_account= next_account_info(iter).unwrap();
    let assets_mint = next_account_info(iter).unwrap();
    let shares_mint = next_account_info(iter).unwrap();
    let user_assets_account = next_account_info(iter).unwrap();
    let authority = next_account_info(iter).unwrap();
    let user_shares_account = next_account_info(iter).unwrap();
 
    let pre = C::new(vault_info, 
        vault_assets_account, 
        Some(assets_mint), 
        Some(shares_mint), 
        Some(user_assets_account), 
        Some(authority), 
        Some(user_shares_account));

    pre.assume_pre();

    let amount = nondet();
    process_redeem_shares(accounts, amount).unwrap();

    let post = C::new(vault_info, 
        vault_assets_account, 
        Some(assets_mint), 
        Some(shares_mint), 
        Some(user_assets_account), 
        Some(authority), 
        Some(user_shares_account));

    clog!(pre, post);
    post.check_post(&pre);
}


#[inline(always)]
pub fn base_process_update_reward<C: CvlrProp>(accounts: &[AccountInfo]) {
    let iter = &mut accounts.iter();
    let vault_info = next_account_info(iter).unwrap();
    let vault_assets_account= next_account_info(iter).unwrap();
 
    let pre = C::new(vault_info, 
                        vault_assets_account, 
                        None, 
                        None, 
                        None, 
                        None, 
                        None);
    pre.assume_pre();

    process_update_reward(accounts).unwrap();

    let post = C::new(vault_info, 
        vault_assets_account, 
        None, 
        None, 
        None, 
        None, 
        None);

    clog!(pre, post);
    post.check_post(&pre);
}
