use crate::{state::Vault, VaultError, VaultResult};

/// Vault operations

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug)]
pub struct VaultEffect {
    pub shares_to_burn: u64,
    pub shares_to_user: u64,
    pub assets_to_vault: u64,
    pub assets_to_user: u64,
    pub assets_to_fee: u64,
}

pub fn vault_deposit_assets(vault: &mut Vault, tkn_amt: u64) -> VaultResult<VaultEffect> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey::Pubkey;

    fn new_test_vault() -> Vault {
        Vault {
            admin: Pubkey::default(),
            shares_mint: Pubkey::default(),
            assets_mint: Pubkey::default(),
            shares: 100u64.into(),
            assets: 100u64.into(),
            vault_assets_account: Pubkey::default(),
            vault_assets_account_bump: 0,
            fee_amount: 0u64.into(),
            fee_bps: 500u64.into(), // 5%
            fee_token_account: Pubkey::default(),
        }
    }

    #[test]
    fn test_vault_deposit_assets() {
        let mut vault = new_test_vault();
        let effect = vault_deposit_assets(&mut vault, 50).unwrap();
        assert_eq!(effect.shares_to_user, 50);
        assert_eq!(effect.assets_to_vault, 50);
        assert_eq!(vault.num_assets(), 150);
        assert_eq!(vault.num_shares(), 150);
    }

    #[test]
    fn test_vault_redeem_shares() {
        let mut vault = new_test_vault();
        let effect = vault_redeem_shares(&mut vault, 40).unwrap();
        assert_eq!(effect.assets_to_user, 40);
        assert_eq!(effect.shares_to_burn, 40);
        assert_eq!(vault.num_assets(), 60);
        assert_eq!(vault.num_shares(), 60);
    }

    #[test]
    fn test_vault_process_slash() {
        let mut vault = new_test_vault();
        let effect = vault_process_slash(&mut vault, 20);
        assert!(effect.is_err());
    }

    #[test]
    fn test_vault_update_reward() {
        let mut vault = new_test_vault();
        let result = vault_update_reward(&mut vault, 150).unwrap();
        assert_eq!(vault.num_assets(), 150);
        assert_eq!(result, VaultEffect::default());
    }
}
