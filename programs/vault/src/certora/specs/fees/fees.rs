use crate::certora::specs::base::{base_deposit_assets, base_deposit_assets_with_fee};
use crate::certora::specs::fees::props::FeeAssessedProp;
use cvlr::prelude::*;

#[rule]
pub fn rule_deposit_assets_with_fee_fees_assessed() {
    base_deposit_assets_with_fee::<FeeAssessedProp>();
}

#[rule]
pub fn rule_deposit_assets_fees_assessed() {
    base_deposit_assets::<FeeAssessedProp>();
}
