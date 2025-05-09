use solana_program::{account_info::AccountInfo, program_error::ProgramError};

use crate::{
    loaders::{DepositContext, RedeemSharesContext},
    operations::{vault_deposit_assets, vault_redeem_shares},
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
        &authority,
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
        &authority,
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
