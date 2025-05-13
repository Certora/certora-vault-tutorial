use crate::certora::specs::base_processor::CvlrProp;
use crate::state::Vault;
use cvlr::mathint::NativeInt;
use cvlr::{cvlr_assert_le, cvlr_assume};
use solana_program::account_info::AccountInfo;
use std::mem::size_of;

pub struct SolvencyInvariant {
    shares_total: NativeInt,
    token_total: NativeInt,
}

mod log {
    use super::*;
    use cvlr::log::cvlr_log_with;
    use cvlr::log::CvlrLog;
    impl CvlrLog for SolvencyInvariant {
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
    fn new(
        vault_info_account: &AccountInfo,
        _vault_assets_account: &AccountInfo,
        _assets_mint: Option<&AccountInfo>,
        _shares_mint: Option<&AccountInfo>,
        _user_assets_account: Option<&AccountInfo>,
        _authority: Option<&AccountInfo>,
        _user_shares_account: Option<&AccountInfo>,
    ) -> Self {
        let data = vault_info_account.try_borrow_data().unwrap();
        let vault = bytemuck::from_bytes::<Vault>(&data[0..size_of::<Vault>()]);

        Self {
            shares_total: vault.num_shares().into(),
            token_total: vault.num_assets().into(),
        }
    }

    fn assume_pre(&self) {
        cvlr_assume!(self.shares_total <= self.token_total);
    }

    fn check_post(&self, _old: &Self) {
        cvlr_assert_le!(self.shares_total, self.token_total);
    }
}

