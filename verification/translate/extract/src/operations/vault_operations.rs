use crate::{state::Vault, VaultError, VaultResult};

/// Vault operations

#[derive(Default)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct VaultEffect {
    pub shares_to_burn: u64,
    pub shares_to_user: u64,
    pub assets_to_vault: u64,
    pub assets_to_user: u64,
    pub assets_to_fee: u64,
}

pub fn vault_deposit_assets(vault: &mut Vault, tkn_amt: u64) -> VaultResult<VaultEffect> {
    //let fee = vault.fee_in_bps()?;
    //require!(fee.is_zero(), crate::errors::VaultError::GuardFail);

    let shares_to_user = vault.convert_assets_to_shares(tkn_amt)?;

    vault.mint_shares(shares_to_user)?;
    vault.add_token(tkn_amt)?;

    Ok(VaultEffect {
        shares_to_user,
        assets_to_vault: tkn_amt,
        ..Default::default()
    })
}

pub fn vault_redeem_shares(vault: &mut Vault, shares_amt: u64) -> VaultResult<VaultEffect> {
    let assets_to_user = vault.convert_shares_to_assets(shares_amt)?;
    vault.burn_shares(shares_amt)?;
    vault.del_token(assets_to_user)?;

    Ok(VaultEffect {
        assets_to_user,
        shares_to_burn: shares_amt,
        ..Default::default()
    })
}

pub fn vault_update_reward(vault: &mut Vault, new_amt: u64) -> VaultResult<VaultEffect> {
    let reward = new_amt
        .checked_sub(vault.num_assets())
        .ok_or(VaultError::MathOverflow)?;

    if reward > 0 {
        vault.add_token(reward)?;
    }

    Ok(VaultEffect::default())
}

pub fn vault_process_slash(vault: &mut Vault, slash_amt: u64) -> VaultResult<VaultEffect> {
    vault.del_token(slash_amt)?;
    Ok(VaultEffect {
        assets_to_user: slash_amt,
        ..Default::default()
    })
}

