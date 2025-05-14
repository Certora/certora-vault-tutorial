use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult};

use crate::{
    loaders::RedeemSharesContext,
    operations::vault_redeem_shares,
    processor::{spl_burn_shares, spl_transfer_assets_from_vault},
    utils::guards::require_ne,
};

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

    require_ne!(
        vault_assets_account.key,
        user_assets_account.key,
        crate::errors::VaultError::SelfTransfer.into()
    );

    let effect = {
        let mut vault = vault_info.get_mut()?;
        vault_redeem_shares(&mut vault, amount)?
    };

    spl_burn_shares(
        effect.shares_to_burn,
        &user_shares_account,
        &shares_mint,
        authority.as_ref(),
        spl_token_program.as_ref(),
    )?;

    spl_transfer_assets_from_vault(
        effect.assets_to_user,
        &vault_assets_account,
        &user_assets_account,
        &assets_mint,
        spl_token_program.as_ref(),
    )?;

    Ok(())
}
