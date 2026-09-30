use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult};

use crate::{loaders::SlashContext, operations::vault_process_slash};

use super::spl_transfer_assets_from_vault;

pub fn process_slash(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let SlashContext {
        vault_info,
        vault_assets_account,
        user_token_account,
        assets_mint,
        authority: _,
        spl_token_program,
    } = SlashContext::load(accounts)?;

    let effects = {
        let mut vault = vault_info.get_mut()?;
        vault_process_slash(&mut vault, amount)?
    };

    let vault_assets_bump = vault_info.get()?.vault_assets_account_bump;
    spl_transfer_assets_from_vault(
        effects.assets_to_user,
        vault_assets_account.as_ref(),
        &user_token_account,
        &assets_mint,
        spl_token_program.as_ref(),
        crate::vault_assets_account_seeds_with_bump!(vault_info.as_ref().key, vault_assets_bump),
    )?;

    Ok(())
}
