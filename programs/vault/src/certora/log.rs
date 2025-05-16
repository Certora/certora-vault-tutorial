use crate::operations::VaultEffect;
use crate::state::Vault;
/// Implementation for cvlr::log::CvlrLog trait
use cvlr::log::cvlr_log_with;

impl cvlr::log::CvlrLog for Vault {
    #[inline(always)]
    fn log(&self, tag: &str, logger: &mut cvlr::log::CvlrLogger) {
        logger.log_scope_start(tag);
        cvlr_log_with("num_shares", &self.num_shares(), logger);
        cvlr_log_with("num_assets", &self.num_assets(), logger);
        logger.log_scope_end(tag);
    }
}

impl cvlr::log::CvlrLog for VaultEffect {
    #[inline(always)]
    fn log(&self, tag: &str, logger: &mut cvlr::log::CvlrLogger) {
        logger.log_scope_start(tag);
        cvlr_log_with("assets_to_fee", &self.assets_to_fee, logger);
        cvlr_log_with("assets_to_user", &self.assets_to_user, logger);
        cvlr_log_with("assets_to_vault", &self.assets_to_vault, logger);
        cvlr_log_with("shares_to_burn", &self.shares_to_burn, logger);
        cvlr_log_with("shares_to_user", &self.shares_to_user, logger);
        logger.log_scope_end(tag);
    }
}
