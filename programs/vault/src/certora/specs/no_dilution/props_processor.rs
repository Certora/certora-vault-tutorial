use crate::certora::specs::base_processor::CvlrProp;
use crate::state::Vault;
use cvlr::mathint::NativeInt;
use cvlr::{cvlr_assert_le, cvlr_assume};
use solana_program::account_info::AccountInfo;
use std::mem::size_of;

pub struct NoDilutionProp {
    shares_total: NativeInt,
    token_total: NativeInt,
}

mod log {
    use super::*;
    use cvlr::log::cvlr_log_with;
    use cvlr::log::CvlrLog;

    impl CvlrLog for NoDilutionProp {
        #[inline(always)]
        fn log(&self, tag: &str, logger: &mut cvlr::log::CvlrLogger) {
            logger.log_scope_start(tag);
            cvlr_log_with("token_total", &self.token_total, logger);
            cvlr_log_with("shares_total", &self.shares_total, logger);
            logger.log_scope_end(tag);
        }
    }
}

fn safe_assumptions(vault: &Vault) {
    cvlr_assume!(cvlr::mathint::is_u64(vault.num_shares()));
    cvlr_assume!(cvlr::mathint::is_u64(vault.num_assets()));
}

/// "no dilution" is a desired property for some operations: the ratio token_total / shares_total cannot decrease.
impl CvlrProp for NoDilutionProp {
    fn new(
        vault_info_account: &AccountInfo,
        _vault_assets_account: &AccountInfo,
        _vault_fee_account: Option<&AccountInfo>,
        _assets_mint: Option<&AccountInfo>,
        _shares_mint: Option<&AccountInfo>,
        _user_assets_account: Option<&AccountInfo>,
        _authority: Option<&AccountInfo>,
        _user_shares_account: Option<&AccountInfo>,
    ) -> Self {
        let data = vault_info_account.try_borrow_data().unwrap();
        let vault = bytemuck::from_bytes::<Vault>(&data[0..size_of::<Vault>()]);
        safe_assumptions(vault);

        Self {
            shares_total: vault.num_shares().into(),
            token_total: vault.num_assets().into(),
        }
    }

    fn assume_pre(&self) {}

    fn check_post(&self, old: &Self) {
        cvlr_assert_le!(
            old.token_total * self.shares_total,
            old.shares_total * self.token_total
        );
    }
}
