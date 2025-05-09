use crate::certora::specs::base_processor::CvlrProp;
use cvlr::mathint::NativeInt;
use cvlr::{cvlr_assume, cvlr_assert_le, cvlr_assert_eq};
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

pub struct VaultConsistencyInvariant {
    vault_token_total: NativeInt,
    account_token_total: NativeInt,
    vault_shares_total: NativeInt,
    mint_shares_total: NativeInt,
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

    impl CvlrLog for VaultConsistencyInvariant {
        #[inline(always)]
        fn log(&self, tag: &str, logger: &mut cvlr::log::CvlrLogger) {
            cvlr_log_with("", &tag, logger);
            cvlr_log_with("\tvault_token_total", &self.vault_token_total, logger);
            cvlr_log_with("\taccount_token_total", &self.account_token_total, logger);
            cvlr_log_with("\tvault_shares_total", &self.vault_shares_total, logger);
            cvlr_log_with("\tmint_shares_total", &self.mint_shares_total, logger);
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

/// Vault's recorded asset/shares amount is consistent with the actual token balances in its associated SPL Token accounts.
impl CvlrProp for VaultConsistencyInvariant {

    fn new(vault_info_account: &AccountInfo,
           vault_assets_account: &AccountInfo,
           _assets_mint: Option<&AccountInfo>,
           shares_mint: Option<&AccountInfo>,
           _user_assets_account: Option<&AccountInfo>,
           _authority: Option<&AccountInfo>,
           _user_shares_account: Option<&AccountInfo>) -> Self { 

        let data = vault_info_account.try_borrow_data().unwrap();
        let vault = bytemuck::from_bytes::<Vault>(&data[0..size_of::<Vault>()]);
        
        // If shares_mint is not passed by the processor function then this property should not be applicable
        cvlr::cvlr_assert!(shares_mint.is_some());

        Self {
            vault_token_total: vault.num_assets().into(),
            account_token_total: cvlr_solana::token::spl_token_account_get_amount(vault_assets_account).into(),
            vault_shares_total: vault.num_shares().into(),
            mint_shares_total: cvlr_solana::token::spl_mint_get_supply(shares_mint.unwrap()).into()
        } 
    }

    fn assume_pre(&self) {
        cvlr_assume!(self.vault_token_total == self.account_token_total);
        cvlr_assume!(self.vault_shares_total == self.mint_shares_total);
    }
    
    fn check_post(&self, _old: &Self) {
        cvlr_assert_eq!(self.vault_token_total, self.account_token_total);
        cvlr_assume!(self.vault_shares_total == self.mint_shares_total);

    }
}