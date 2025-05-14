use crate::certora::specs::base_processor::CvlrProp;
use crate::state::Vault;
use cvlr::mathint::NativeInt;
use cvlr::{cvlr_assert_eq, cvlr_assert_le, cvlr_assume};
use solana_program::account_info::AccountInfo;
use std::mem::size_of;

pub struct VaultConsistencyInvariant {
    vault_assets: NativeInt,
    vault_shares: NativeInt,
    account_tokens: NativeInt,
    mint_shares: Option<NativeInt>,
}

mod log {
    use super::*;
    use cvlr::log::cvlr_log_with;
    use cvlr::log::CvlrLog;

    impl CvlrLog for VaultConsistencyInvariant {
        #[inline(always)]
        fn log(&self, tag: &str, logger: &mut cvlr::log::CvlrLogger) {
            cvlr_log_with("", &tag, logger);
            cvlr_log_with("\tvault_assets", &self.vault_assets, logger);
            cvlr_log_with("\tvault_shares", &self.vault_shares, logger);
            cvlr_log_with("\taccount_tokens", &self.account_tokens, logger);
            cvlr_log_with("\tmint_shares", &self.mint_shares, logger);
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

        Self {
            vault_assets: vault.num_assets().into(),
            vault_shares: vault.num_shares().into(),
            account_tokens: cvlr_solana::token::spl_token_account_get_amount(vault_assets_account)
                .into(),
            mint_shares: match shares_mint {
                Some(mint) => Some(cvlr_solana::token::spl_mint_get_supply(mint).into()),
                None => None,
            },
        }
    }

    fn assume_pre(&self) {
        cvlr_assume!(self.vault_assets <= self.account_tokens);
        if let Some(mint_shares) = self.mint_shares {
            cvlr_assume!(self.vault_shares == mint_shares);
        }
    }

    fn check_post(&self, _old: &Self) {
        cvlr_assert_le!(self.vault_assets, self.account_tokens);
        if let Some(mint_shares) = self.mint_shares {
            cvlr_assert_eq!(self.vault_shares, mint_shares);
        }
    }
}
