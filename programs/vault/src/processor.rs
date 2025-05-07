use solana_program::{account_info::AccountInfo, program_error::ProgramError};

use crate::{
    loaders::{DepositContext, RedeemSharesContext, UpdateRewardContext},
    operations::{vault_deposit_assets, vault_redeem_shares, vault_update_reward},
};

pub fn process_deposit(accounts: &[AccountInfo], amount: u64) -> Result<(), ProgramError> {
    let DepositContext {
        vault_info,
        vault_assets_account,
        assets_mint,
        shares_mint,
        user_assets_account,
        authority,
        user_shares_account,
        spl_token_program,
    } = DepositContext::load(accounts)?;

    let effect = {
        let mut vault = vault_info.get_mut()?;
        vault_deposit_assets(&mut *vault, amount).map_err(|e| -> ProgramError { e.into() })?
    };

    spl_transfer_assets_to_vault(
        effect.assets_to_vault,
        &vault_assets_account,
        &user_assets_account,
        &assets_mint,
        authority.as_ref(),
        &spl_token_program,
    )?;

    spl_mint_shares(
        effect.shares_to_user,
        &user_shares_account,
        &shares_mint,
        &spl_token_program,
    )?;

    Ok(())
}

pub fn process_redeem_shares(accounts: &[AccountInfo], amount: u64) -> Result<(), ProgramError> {
    let context = RedeemSharesContext::load(accounts)?;
    let RedeemSharesContext {
        vault_info,
        vault_assets_account,
        assets_mint,
        shares_mint,
        user_shares_account,
        authority,
        user_assets_account,
        spl_token_program,
    } = context;

    let effect = {
        let mut vault = vault_info.get_mut()?;
        vault_redeem_shares(&mut *vault, amount)?
    };

    spl_burn_shares(
        effect.shares_to_burn,
        &shares_mint,
        &user_shares_account,
        authority.as_ref(),
        &spl_token_program,
    )?;

    spl_transfer_assets_to_user(
        effect.assets_to_user,
        &vault_assets_account,
        &user_assets_account,
        &assets_mint,
        &spl_token_program,
    )?;

    Ok(())
}

pub fn process_update_reward(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    let context = UpdateRewardContext::load(accounts)?;

    // This instruction is permissionless. Anyone can run it to update vault state.
    // IRL it should be limited to once per epoch 

    let UpdateRewardContext {
        vault_info,
        vault_assets_account,
    } = context;

    let vault_asset_account_amount = spl_token_account_amount(&vault_assets_account)?;

    let _effect = {
        let mut vault = vault_info.get_mut()?;
        vault_update_reward(&mut *vault, vault_asset_account_amount)?;
    };

    Ok(())
}

pub fn spl_transfer_assets_to_vault(
    _amount: u64,
    _vault_assets: &AccountInfo,
    _user_assets: &AccountInfo,
    _mint: &AccountInfo,
    _authority: &AccountInfo,
    _spl_token_program: &AccountInfo,
) -> Result<(), ProgramError> {
    Ok(())
}

pub fn spl_mint_shares(
    _amount: u64,
    _user_shares_account: &AccountInfo,
    _mint: &AccountInfo,
    _spl_token_program: &AccountInfo,
) -> Result<(), ProgramError> {
    // CPI call. Use PDA as a mint authority
    Ok(())
}

pub fn spl_burn_shares(
    _amount: u64,
    _user_shares_account: &AccountInfo,
    _mint: &AccountInfo,
    _authority: &AccountInfo,
    _spl_token_program: &AccountInfo,
) -> Result<(), ProgramError> {
    // CPI call. Use PDA as a mint authority
    Ok(())
}

pub fn spl_transfer_assets_to_user(
    _amount: u64,
    _vault_assets: &AccountInfo,
    _user_assets: &AccountInfo,
    _mint: &AccountInfo,
    _spl_token_program: &AccountInfo,
) -> Result<(), ProgramError> {
    // Use PDA as vault assets owner
    Ok(())
}

pub fn spl_token_account_amount(_info: &AccountInfo) -> Result<u64, ProgramError> {
    // read amount value from the account
    Ok(0)
}
