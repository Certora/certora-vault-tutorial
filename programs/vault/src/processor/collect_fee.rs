pub fn process_collect_fee(accounts: &[AccountInfo]) -> ProgramResult {
    let CollectFeeContext {
        vault_info,
        vault_assets_account,
        assets_mint,
        fee_collect_account,
        authority: _,
        spl_token_program,
    } = CollectFeeContext::load(accounts)?;

    let effect = {
        let mut vault = vault_info.get_mut()?;
        vault_collect_fee(&mut vault)?
    };

    let vault_assets_bump = vault_info.get()?.vault_assets_account_bump;
    spl_transfer_assets_from_vault(
        effect.assets_to_user,
        &vault_assets_account,
        &fee_collect_account,
        &assets_mint,
        spl_token_program.as_ref(),
        crate::vault_assets_account_seeds_with_bump!(vault_info.as_ref().key, vault_assets_bump),
    )?;

    Ok(())
}