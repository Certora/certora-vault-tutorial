use crate::certora::specs::base::{CvlrProp, OperationParams};
use crate::state::Vault;
use crate::operations::VaultEffect;
use cvlr::mathint::NativeInt;
use cvlr::{cvlr_assume, cvlr_assert_le, cvlr_assert_gt};

pub struct SolvencyInvariant {
    shares_total: NativeInt, 
    token_total: NativeInt,
} 

pub struct NoDilutionProp {
    shares_total: NativeInt, 
    token_total: NativeInt,
} 

pub struct FeeAssessedProp {
    fee_bps: NativeInt
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

    impl CvlrLog for NoDilutionProp {
        #[inline(always)]
        fn log(&self, tag: &str, logger: &mut cvlr::log::CvlrLogger) {
            use cvlr::log::cvlr_log_with;
            cvlr_log_with("", &tag, logger);
            cvlr_log_with("\ttoken_total", &self.token_total, logger);
            cvlr_log_with("\tshares_total", &self.shares_total, logger);
        }
    }

    impl CvlrLog for FeeAssessedProp {
        #[inline(always)]
        fn log(&self, _tag: &str, _logger: &mut cvlr::log::CvlrLogger) {}
    }

    impl CvlrLog for VaultEffect {
        #[inline(always)]
        fn log(&self, tag: &str, logger: &mut cvlr::log::CvlrLogger) {
            use cvlr::log::cvlr_log_with;
            cvlr_log_with("", &tag, logger);
            cvlr_log_with("\tassets_to_fee", &self.assets_to_fee, logger);
            cvlr_log_with("\tassets_to_user", &self.assets_to_user, logger);
            cvlr_log_with("\tassets_to_vault", &self.assets_to_vault, logger);
            cvlr_log_with("\tshares_to_burn", &self.shares_to_burn, logger);
            cvlr_log_with("\tshares_to_user", &self.shares_to_user, logger);
        }
    }

}


/// Solvency is an invariant: the vault can never have more shares than tokens. 
impl CvlrProp for SolvencyInvariant {
    fn new(vault: &Vault) -> Self { 
        Self {
            shares_total: vault.num_shares().into(),
            token_total: vault.num_assets().into()
        } 
    }

    fn assume_pre(&self) {
        cvlr_assume!(self.shares_total <= self.token_total);
    }
    
    fn check_post(&self, _old: &Self, _params: OperationParams, _effect: VaultEffect) {
        cvlr_assert_le!(self.shares_total, self.token_total);
    }
}

/// "no dilution" is a desired property for some operations: the ratio token_total / shares_total cannot decrease.
impl CvlrProp for NoDilutionProp {
    fn new(vault: &Vault) -> Self { 
        Self {
            shares_total: vault.num_shares().into(),
            token_total: vault.num_assets().into()
        } 
    }

    fn assume_pre(&self) {}
    
    fn check_post(&self, old: &Self, _params: OperationParams, _effect: VaultEffect) {
        cvlr_assert_le!(old.token_total * self.shares_total, old.shares_total * self.token_total);
    }
}

/// property for `vault_deposit_assets_with_fee`
impl CvlrProp for FeeAssessedProp {
    fn new(vault: &Vault) -> Self { 
        Self {
            fee_bps: u64::from(vault.fee_bps).into()
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