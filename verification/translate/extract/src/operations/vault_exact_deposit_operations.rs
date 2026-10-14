use crate::{state::Vault, utils::math::FeeBps, VaultResult};

use super::VaultEffect;

pub fn vault_deposit_assets_exact(vault: &mut Vault, tkn_amt: u64) -> VaultResult<VaultEffect> {
    let shares_to_user = vault.convert_assets_to_shares(tkn_amt)?;
    let assets_to_vault = vault.convert_shares_to_assets(shares_to_user)?;

    vault.mint_shares(shares_to_user)?;
    vault.add_token(assets_to_vault)?;

    Ok(VaultEffect {
        shares_to_user,
        assets_to_vault,
        ..Default::default()
    })
}

pub fn vault_deposit_assets_with_fee_exact(
    vault: &mut Vault,
    tkn_amt: u64,
) -> VaultResult<VaultEffect> {
    let fee_bps: FeeBps = vault.fee_in_bps()?;
    // -- maximum possible fee
    let gross = fee_bps.apply(tkn_amt)?;

    let shares_to_user = vault.convert_assets_to_shares(gross.net_amount)?;
    let assets_to_vault = vault.convert_shares_to_assets(shares_to_user)?;

    // -- compute fee based on actual use
    let actual_gross = fee_bps.apply(assets_to_vault)?;

    Ok(VaultEffect {
        shares_to_user,
        assets_to_vault,
        assets_to_fee: actual_gross.fee,
        ..Default::default()
    })
}
