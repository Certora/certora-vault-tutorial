use super::{base::*, props::*};
use cvlr::prelude::*;

#[rule]
pub fn rule_fees_assessed() {
    base_deposit_assets_with_fee::<FeeAssessedProp>();
}





