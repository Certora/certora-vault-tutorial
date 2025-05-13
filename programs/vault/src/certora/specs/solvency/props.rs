use crate::certora::specs::base::{CvlrProp, OperationParams};
use crate::operations::VaultEffect;
use crate::state::Vault;
use cvlr::mathint::NativeInt;
use cvlr::{cvlr_assert_le, cvlr_assume};

pub struct SolvencyInvariant {
    shares_total: NativeInt,
    token_total: NativeInt,
}

mod log {
    use super::*;
    use cvlr::log::CvlrLog;
    impl CvlrLog for SolvencyInvariant {
        #[inline(always)]
        fn log(&self, tag: &str, logger: &mut cvlr::log::CvlrLogger) {
            use cvlr::log::cvlr_log_with;
            cvlr_log_with("", &tag, logger);
            cvlr_log_with("\ttoken_total", &self.token_total, logger);
            cvlr_log_with("\tshares_total", &self.shares_total, logger);
        }
    }
}

/// Solvency is an invariant: the vault can never have more shares than tokens.
impl CvlrProp for SolvencyInvariant {
    fn new(vault: &Vault) -> Self {
        Self {
            shares_total: vault.num_shares().into(),
            token_total: vault.num_assets().into(),
        }
    }

    fn assume_pre(&self) {
        cvlr_assume!(self.shares_total <= self.token_total);
    }

    fn check_post(&self, _old: &Self, _params: OperationParams, _effect: VaultEffect) {
        cvlr_assert_le!(self.shares_total, self.token_total);
    }
}
