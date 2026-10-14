use crate::{state::Vault, utils::math::FeeBps, VaultResult};

use super::VaultEffect;

pub fn vault_deposit_assets_with_fee(vault: &mut Vault, tkn_amt: u64) -> VaultResult<VaultEffect> {
    let fee_bps: FeeBps = vault.fee_in_bps()?;
    let gross = fee_bps.apply(tkn_amt)?;

    let shares_to_user = vault.convert_assets_to_shares(gross.net_amount)?;

    vault.mint_shares(shares_to_user)?;
    vault.add_token(gross.net_amount)?;

    Ok(VaultEffect {
        shares_to_user,
        assets_to_vault: gross.net_amount,
        assets_to_fee: gross.fee,
        ..Default::default()
    })
}

pub fn vault_collect_fee(vault: &mut Vault) -> VaultResult<VaultEffect> {
    let fee_amount = vault.fee_amount();

    vault.clear_fee_amount();

    Ok(VaultEffect {
        assets_to_user: fee_amount,
        ..Default::default()
    })
}
