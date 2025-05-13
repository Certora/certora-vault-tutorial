use crate::certora::specs::base::base_deposit_assets_with_fee;
use crate::certora::specs::fees::props::FeeAssessedProp;
use cvlr::prelude::*;

#[rule]
pub fn rule_fees_assessed() {
    base_deposit_assets_with_fee::<FeeAssessedProp>();
}
