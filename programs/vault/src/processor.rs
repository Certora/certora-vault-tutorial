use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
};

use crate::{
    loaders::{DepositContext, RedeemSharesContext, UpdateRewardContext},
    operations::{vault_deposit_assets, vault_redeem_shares, vault_update_reward},
    utils::guards::require_ne,
};

pub fn process_deposit(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
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

    require_ne!(vault_assets_account.key, user_assets_account.key, VaultError::SelfTransfer);

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
        spl_token_program.as_ref(),
    )?;

    spl_mint_shares(
        effect.shares_to_user,
        &user_shares_account,
        &shares_mint,
        authority.as_ref(),
        spl_token_program.as_ref(),
    )?;

    Ok(())
}

pub fn process_redeem_shares(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
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

    require_ne!(vault_assets_account.key, user_assets_account.key, VaultError::SelfTransfer);

    let effect = {
        let mut vault = vault_info.get_mut()?;
        vault_redeem_shares(&mut *vault, amount)?
    };

    spl_burn_shares(
        effect.shares_to_burn,
        &shares_mint,
        &user_shares_account,
        authority.as_ref(),
        spl_token_program.as_ref(),
    )?;

    spl_transfer_assets_to_user(
        effect.assets_to_user,
        &vault_assets_account,
        &user_assets_account,
        &assets_mint,
        authority.as_ref(),
        spl_token_program.as_ref(),
    )?;

    Ok(())
}

pub fn process_update_reward(accounts: &[AccountInfo]) -> ProgramResult {
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

#[cfg_attr(feature = "certora", cvlr::mock_fn(with=crate::certora::mocks::processor::spl_transfer_assets_to_vault))]
pub fn spl_transfer_assets_to_vault(
    _amount: u64,
    _vault_assets: &AccountInfo,
    _user_assets: &AccountInfo,
    _mint: &AccountInfo,
    _authority: &AccountInfo,
    _spl_token_program: &AccountInfo,
) -> ProgramResult {
    // CPI call
    Ok(())
}

#[cfg_attr(feature = "certora", cvlr::mock_fn(with=crate::certora::mocks::processor::spl_mint_shares))]
pub fn spl_mint_shares(
    _amount: u64,
    _user_shares_account: &AccountInfo,
    _mint: &AccountInfo,
    _authority: &AccountInfo,
    _spl_token_program: &AccountInfo,
) -> ProgramResult {
    // CPI call. Use PDA as a mint authority
    Ok(())
}

#[cfg_attr(feature = "certora", cvlr::mock_fn(with=crate::certora::mocks::processor::spl_burn_shares))]
pub fn spl_burn_shares(
    _amount: u64,
    _user_shares_account: &AccountInfo,
    _mint: &AccountInfo,
    _authority: &AccountInfo,
    _spl_token_program: &AccountInfo,
) -> ProgramResult {
    // CPI call. Use PDA as a mint authority
    Ok(())
}

#[cfg_attr(feature = "certora", cvlr::mock_fn(with=crate::certora::mocks::processor::spl_transfer_assets_to_user))]
pub fn spl_transfer_assets_to_user(
    _amount: u64,
    _vault_assets: &AccountInfo,
    _user_assets: &AccountInfo,
    _mint: &AccountInfo,
    _authority: &AccountInfo,
    _spl_token_program: &AccountInfo,
) -> ProgramResult {
    // Use PDA as vault assets owner
    Ok(())
}

#[cfg_attr(feature = "certora", cvlr::mock_fn(with=crate::certora::mocks::processor::spl_token_account_amount))]
pub fn spl_token_account_amount(_info: &AccountInfo) -> Result<u64, ProgramError> {
    // CPI call. Read amount value from the account
    Ok(0)
}
