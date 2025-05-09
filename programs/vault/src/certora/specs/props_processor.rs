use crate::certora::specs::base_processor::CvlrProp;
use cvlr::mathint::NativeInt;
use cvlr::{cvlr_assume, cvlr_assert_le};
use crate::state::Vault;
use std::mem::size_of;
use solana_program::account_info::AccountInfo;

pub struct SolvencyInvariant {
    shares_total: NativeInt, 
    token_total: NativeInt,
} 

pub struct NoDilutionProp {
    shares_total: NativeInt, 
    token_total: NativeInt,
} 

mod log {
    use super::*;
    use cvlr::log::CvlrLog;
    use cvlr::log::cvlr_log_with;
    impl CvlrLog for SolvencyInvariant {
        #[inline(always)]
        fn log(&self, tag: &str, logger: &mut cvlr::log::CvlrLogger) {
        
            cvlr_log_with("", &tag, logger);
            cvlr_log_with("\ttoken_total", &self.token_total, logger);
            cvlr_log_with("\tshares_total", &self.shares_total, logger);
        }
    }

    impl CvlrLog for NoDilutionProp {
        #[inline(always)]
        fn log(&self, tag: &str, logger: &mut cvlr::log::CvlrLogger) {
            cvlr_log_with("", &tag, logger);
            cvlr_log_with("\ttoken_total", &self.token_total, logger);
            cvlr_log_with("\tshares_total", &self.shares_total, logger);
        }
    }
}


/// Solvency is an invariant: the vault can never have more shares than tokens. 
impl CvlrProp for SolvencyInvariant {
    fn new(vault_info_account: &AccountInfo,
           _vault_assets_account: &AccountInfo,
           _assets_mint: Option<&AccountInfo>,
           _shares_mint: Option<&AccountInfo>,
           _user_assets_account: Option<&AccountInfo>,
           _authority: Option<&AccountInfo>,
           _user_shares_account: Option<&AccountInfo>) -> Self { 

        let data = vault_info_account.try_borrow_data().unwrap();
        let vault = bytemuck::from_bytes::<Vault>(&data[0..size_of::<Vault>()]);
        
        Self {
            shares_total: vault.num_shares().into(),
            token_total: vault.num_assets().into()
        } 
    }

    fn assume_pre(&self) {
        cvlr_assume!(self.shares_total <= self.token_total);
    }
    
    fn check_post(&self, _old: &Self) {
        cvlr_assert_le!(self.shares_total, self.token_total);
    }
}

/// "no dilution" is a desired property for some operations: the ratio token_total / shares_total cannot decrease.
impl CvlrProp for NoDilutionProp {

    fn new(vault_info_account: &AccountInfo,
           _vault_assets_account: &AccountInfo,
           _assets_mint: Option<&AccountInfo>,
           _shares_mint: Option<&AccountInfo>,
           _user_assets_account: Option<&AccountInfo>,
           _authority: Option<&AccountInfo>,
           _user_shares_account: Option<&AccountInfo>) -> Self { 

        let data = vault_info_account.try_borrow_data().unwrap();
        let vault = bytemuck::from_bytes::<Vault>(&data[0..size_of::<Vault>()]);

        Self {
            shares_total: vault.num_shares().into(),
            token_total: vault.num_assets().into()
        } 
    }

    fn assume_pre(&self) {}
    
    fn check_post(&self, old: &Self) {
        cvlr_assert_le!(old.token_total * self.shares_total, old.shares_total * self.token_total);
    }
}