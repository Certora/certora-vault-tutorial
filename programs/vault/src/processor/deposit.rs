use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
};

use crate::{
    loaders::DepositContext,
    operations::vault_deposit_assets,
    processor::{spl_mint_shares, spl_transfer_assets_from_user},
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

    require_ne!(
        vault_assets_account.as_ref().key,
        user_assets_account.key,
        crate::errors::VaultError::SelfTransfer.into()
    );

    let effect = {
        let mut vault = vault_info.get_mut()?;
        vault_deposit_assets(&mut vault, amount).map_err(|e| -> ProgramError { e.into() })?
    };

    spl_transfer_assets_from_user(
        effect.assets_to_vault,
        vault_assets_account.as_ref(),
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
