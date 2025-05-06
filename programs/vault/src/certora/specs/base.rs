use cvlr::log::CvlrLog;
use cvlr::clog;
use cvlr::nondet;
use crate::state::Vault;
use crate::operations::*;

pub struct OperationParams {
    // it can be either tokens or shares depending on the operation
    pub amount: u64,
}

pub trait CvlrProp: CvlrLog {
    fn new(pool: &Vault) -> Self;
    fn assume_pre(&self);
    fn check_post(&self, old: &Self, params: OperationParams, effect: VaultEffect);
}


#[inline(always)]
pub fn base_deposit_assets<C: CvlrProp>() {
    let mut vault: Vault = nondet();
    let pre = C::new(&vault);
    pre.assume_pre();

    let token_amount = nondet();
    let effect = vault_deposit_assets(&mut vault, token_amount).unwrap();
  
    let post = C::new(&vault);
    clog!(pre, post);
    post.check_post(&pre, OperationParams { amount: token_amount }, effect);
}

#[inline(always)]
pub fn base_deposit_assets_with_fee<C: CvlrProp>() {
    let mut vault: Vault = nondet();
    let pre = C::new(&vault);
    pre.assume_pre();

    let token_amount = nondet();
    let effect = vault_deposit_assets_with_fee(&mut vault, token_amount).unwrap();
  
    let post = C::new(&vault);
    clog!(pre, post);
    post.check_post(&pre, OperationParams { amount: token_amount }, effect);
}


#[inline(always)]
pub fn base_redeem_shares<C: CvlrProp>() {
    let mut vault: Vault = nondet();
    let pre = C::new(&vault);
    pre.assume_pre();

    let shares_amount = nondet();
    let effect = vault_redeem_shares(&mut vault, shares_amount).unwrap();
  
    let post = C::new(&vault);
    clog!(pre, post);
    post.check_post(&pre, OperationParams { amount: shares_amount }, effect);
}

#[inline(always)]
pub fn base_update_reward<C: CvlrProp>() {
    let mut vault: Vault = nondet();
    let pre = C::new(&vault);
    pre.assume_pre();

    let token_amount = nondet();
    let effect = vault_update_reward(&mut vault, token_amount).unwrap();
  
    let post = C::new(&vault);
    clog!(pre, post);
    post.check_post(&pre, OperationParams { amount: token_amount }, effect);
}

#[inline(always)]
pub fn base_process_slash<C: CvlrProp>() {
    let mut vault: Vault = nondet();
    let pre = C::new(&vault);
    pre.assume_pre();

    let token_amount = nondet();
    let effect = vault_process_slash(&mut vault, token_amount).unwrap();
  
    let post = C::new(&vault);
    clog!(pre, post);
    post.check_post(&pre, OperationParams { amount: token_amount }, effect);
}