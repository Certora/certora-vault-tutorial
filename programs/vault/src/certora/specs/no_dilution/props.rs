use crate::certora::specs::base::{CvlrProp, OperationParams};
use crate::operations::VaultEffect;
use crate::state::Vault;
use cvlr::cvlr_assert_le;
use cvlr::mathint::NativeInt;

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

/// "no dilution" is a desired property for some operations: the ratio token_total / shares_total cannot decrease.
impl CvlrProp for NoDilutionProp {
    fn new(vault: &Vault) -> Self {
        Self {
            shares_total: vault.num_shares().into(),
            token_total: vault.num_assets().into(),
        }
    }

    fn assume_pre(&self) {}

    fn check_post(&self, old: &Self, _params: OperationParams, _effect: VaultEffect) {
        cvlr_assert_le!(
            old.token_total * self.shares_total,
            old.shares_total * self.token_total
        );
    }
}
