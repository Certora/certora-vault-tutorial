use crate::certora::constants::MAX_FEE_BPS;
/// Implementations for cvlr::nondet::Nondet trait
use crate::state::Vault;
use cvlr::nondet::{nondet, nondet_with};
use cvlr_solana::cvlr_nondet_pubkey;

impl cvlr::nondet::Nondet for Vault {
    fn nondet() -> Self {
        Self {
            admin: cvlr_nondet_pubkey(),
            slash_admin: cvlr_nondet_pubkey(),
            shares_mint: cvlr_nondet_pubkey(),
            assets_mint: cvlr_nondet_pubkey(),
            shares: u64::nondet().into(),
            assets: u64::nondet().into(),
            vault_assets_account: cvlr_nondet_pubkey(),
            vault_assets_account_bump: nondet(),
            fee_bps: nondet_with(|x: &u64| *x <= MAX_FEE_BPS).into(),
            fee_amount: u64::nondet().into(),
            fee_token_account: cvlr_nondet_pubkey(),
        }
    }
}
