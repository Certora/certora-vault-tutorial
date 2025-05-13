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

    spl_transfer_assets_from_vault(
        effect.assets_to_user,
        &vault_assets_account,
        &fee_collect_account,
        &assets_mint,
        spl_token_program.as_ref(),
    )?;

    Ok(())
}