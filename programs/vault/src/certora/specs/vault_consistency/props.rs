use crate::certora::specs::base_processor::CvlrProp;
use crate::state::Vault;
use cvlr::mathint::NativeInt;
use cvlr::{cvlr_assert_eq, cvlr_assume};
use solana_program::account_info::AccountInfo;
use std::mem::size_of;

pub struct VaultConsistencyInvariant {
    vault_token_total: NativeInt,
    account_token_total: NativeInt,
    vault_shares_total: NativeInt,
    mint_shares_total: NativeInt,
}

mod log {
    use super::*;
    use cvlr::log::cvlr_log_with;
    use cvlr::log::CvlrLog;

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

/// Vault's recorded asset/shares amount is consistent with the actual token balances in its associated SPL Token accounts.
impl CvlrProp for VaultConsistencyInvariant {
    fn new(
        vault_info_account: &AccountInfo,
        vault_assets_account: &AccountInfo,
        _assets_mint: Option<&AccountInfo>,
        shares_mint: Option<&AccountInfo>,
        _user_assets_account: Option<&AccountInfo>,
        _authority: Option<&AccountInfo>,
        _user_shares_account: Option<&AccountInfo>,
    ) -> Self {
        let data = vault_info_account.try_borrow_data().unwrap();
        let vault = bytemuck::from_bytes::<Vault>(&data[0..size_of::<Vault>()]);

        // If shares_mint is not passed by the processor function then this property should not be applicable
        cvlr::cvlr_assert!(shares_mint.is_some());

        Self {
            vault_token_total: vault.num_assets().into(),
            account_token_total: cvlr_solana::token::spl_token_account_get_amount(
                vault_assets_account,
            )
            .into(),
            vault_shares_total: vault.num_shares().into(),
            mint_shares_total: cvlr_solana::token::spl_mint_get_supply(shares_mint.unwrap()).into(),
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
