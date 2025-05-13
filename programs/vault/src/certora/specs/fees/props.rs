use crate::certora::specs::base::{CvlrProp, OperationParams};
use crate::operations::VaultEffect;
use crate::state::Vault;
use cvlr::mathint::NativeInt;
use cvlr::{cvlr_assert_gt, cvlr_assert_le};

pub struct FeeAssessedProp {
    fee_bps: NativeInt,
}

mod log {
    use super::*;
    use cvlr::log::CvlrLog;
    impl CvlrLog for FeeAssessedProp {
        #[inline(always)]
        fn log(&self, _tag: &str, _logger: &mut cvlr::log::CvlrLogger) {}
    }
}

/// property for `vault_deposit_assets_with_fee`
impl CvlrProp for FeeAssessedProp {
    fn new(vault: &Vault) -> Self {
        Self {
            fee_bps: u64::from(vault.fee_bps).into(),
        }
    }

    fn assume_pre(&self) {}

    fn check_post(&self, _old: &Self, params: OperationParams, effect: VaultEffect) {
        let tokens_amount = params.amount;
        let fee_bps = self.fee_bps;
        cvlr::clog!(tokens_amount, fee_bps, effect);
        cvlr_assert_le!(effect.assets_to_fee, tokens_amount);
        if fee_bps > 0u64.into() {
            cvlr_assert_gt!(effect.assets_to_fee, 0);
        }
    }
}
