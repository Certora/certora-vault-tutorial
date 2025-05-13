use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult};

use crate::{loaders::UpdateRewardContext, operations::vault_update_reward};

use super::spl_token_account_amount;

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
        vault_update_reward(&mut vault, vault_asset_account_amount)?
    };

    Ok(())
}
