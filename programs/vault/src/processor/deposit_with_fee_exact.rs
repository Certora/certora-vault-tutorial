use crate::{
    operations::vault_deposit_assets_with_fee_exact,
    processor::{spl_mint_shares, spl_transfer_assets_from_user},
    utils::guards::require_ne,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
};

use crate::loaders::DepositWithFeeContext;

pub fn process_deposit_with_fee_exact(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let DepositWithFeeContext {
        vault_info,
        vault_assets_account,
        vault_fee_account,
        assets_mint,
        shares_mint,
        user_assets_account,
        authority,
        user_shares_account,
        spl_token_program,
    } = DepositWithFeeContext::load(accounts)?;

    require_ne!(
        vault_assets_account.key,
        user_assets_account.key,
        crate::errors::VaultError::SelfTransfer.into()
    );

    let effect = {
        let mut vault = vault_info.get_mut()?;
        vault_deposit_assets_with_fee_exact(&mut vault, amount)
            .map_err(|e| -> ProgramError { e.into() })?
    };

    // -- transfer assets into vault
    spl_transfer_assets_from_user(
        effect.assets_to_vault,
        &vault_assets_account,
        &user_assets_account,
        &assets_mint,
        authority.as_ref(),
        spl_token_program.as_ref(),
    )?;

    // -- transfer fee from user to vault
    spl_transfer_assets_from_user(
        effect.assets_to_fee,
        &vault_fee_account,
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
