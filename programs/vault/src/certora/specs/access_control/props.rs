use crate::certora::specs::base_processor::CvlrProp;
use crate::state::Vault;
use cvlr::cvlr_assert;
use cvlr_solana::pubkey::Pk;
use solana_program::{account_info::AccountInfo, pubkey::Pubkey};
use std::mem::size_of;

pub struct AccessControlProcessDeposit {
    vault_assets_account_key: Pubkey,
    vault_assets_mint_key: Pubkey,
    vault_shares_mint_key: Pubkey,
    assets_account_key: Pubkey,
    assets_mint_key: Pubkey,
    shares_mint_key: Pubkey,
}

mod log {
    use super::*;
    use cvlr::log::cvlr_log_with;
    use cvlr::log::CvlrLog;

    impl CvlrLog for AccessControlProcessDeposit {
        #[inline(always)]
        fn log(&self, tag: &str, logger: &mut cvlr::log::CvlrLogger) {
            cvlr_log_with("", &tag, logger);
            cvlr_log_with(
                "\tvault_assets_account_key",
                &Pk(&self.vault_assets_account_key),
                logger,
            );
            cvlr_log_with(
                "\tvault_assets_mint_key",
                &Pk(&self.vault_assets_mint_key),
                logger,
            );
            cvlr_log_with(
                "\tvault_shares_mint_key",
                &Pk(&self.vault_shares_mint_key),
                logger,
            );
            cvlr_log_with(
                "\tassets_account_key",
                &Pk(&self.assets_account_key),
                logger,
            );
            cvlr_log_with("\tassets_mint_key", &Pk(&self.assets_mint_key), logger);
            cvlr_log_with("\tshares_mint_key", &Pk(&self.shares_mint_key), logger);
        }
    }
}

impl CvlrProp for AccessControlProcessDeposit {
    fn new(
        vault_info_account: &AccountInfo,
        vault_assets_account: &AccountInfo,
        assets_mint: Option<&AccountInfo>,
        shares_mint: Option<&AccountInfo>,
        _user_assets_account: Option<&AccountInfo>,
        _authority: Option<&AccountInfo>,
        _user_shares_account: Option<&AccountInfo>,
    ) -> Self {
        let data = vault_info_account.try_borrow_data().unwrap();
        let vault = bytemuck::from_bytes::<Vault>(&data[0..size_of::<Vault>()]);

        let vault_assets_mint_key = vault.assets_mint;
        let vault_shares_mint_key = vault.shares_mint;
        let vault_assets_account_key = vault.vault_assets_account;

        cvlr::cvlr_assert!(assets_mint.is_some());
        cvlr::cvlr_assert!(shares_mint.is_some());
        let assets_account_key = *vault_assets_account.key;
        let assets_mint_key = *assets_mint.unwrap().key;
        let shares_mint_key = *shares_mint.unwrap().key;

        Self {
            vault_assets_account_key,
            vault_assets_mint_key,
            vault_shares_mint_key,
            assets_account_key,
            assets_mint_key,
            shares_mint_key,
        }
    }

    fn assume_pre(&self) {}

    fn check_post(&self, _old: &Self) {
        cvlr_assert!(self.vault_assets_account_key == self.assets_account_key);
        cvlr_assert!(self.vault_assets_mint_key == self.assets_mint_key);
        cvlr_assert!(self.vault_shares_mint_key == self.shares_mint_key);
    }
}
