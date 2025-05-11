/// Implementation for cvlr::log::CvlrLog trait

use cvlr::log::cvlr_log_with;
use crate::state::Vault;

impl cvlr::log::CvlrLog for Vault {
    fn log(&self, tag: &str, logger: &mut cvlr::log::CvlrLogger) {
        cvlr_log_with(tag, &"BEGIN", logger);
        cvlr_log_with("num_shares", &self.num_shares(), logger);
        cvlr_log_with("num_assets", &self.num_assets(), logger);
    }
}