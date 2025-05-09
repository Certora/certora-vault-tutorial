use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
};

pub fn spl_transfer_assets_to_vault<'a>(
    amount: u64,
    vault_assets: &AccountInfo<'a>,
    user_assets: &AccountInfo<'a>,
    _mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    _spl_token_program: &AccountInfo<'a>,
) -> ProgramResult {
    // We can use `spl_token_program` to know which token version we are and call either spl_token_transfer or spl_token_2022_transfer.
    // However, our mocks for spl_token_transfer and spl_token_2022_transfer are the same.
    cvlr_solana::token::spl_token_2022_transfer(user_assets, vault_assets, authority, amount)
}

pub fn spl_mint_shares<'a>(
    amount: u64,
    user_shares_account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    _spl_token_program: &AccountInfo<'a>,
) -> ProgramResult {
    cvlr_solana::token::spl_mint_to(mint, user_shares_account, authority, amount)
}

pub fn spl_burn_shares<'a>(
    amount: u64,
    user_shares_account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    _spl_token_program: &AccountInfo<'a>,
) -> ProgramResult {
    cvlr_solana::token::spl_burn(mint, user_shares_account, authority, amount)
}

pub fn spl_transfer_assets_to_user<'a>(
    amount: u64,
    vault_assets: &AccountInfo<'a>,
    user_assets: &AccountInfo<'a>,
    _mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    _spl_token_program: &AccountInfo<'a>,
) -> ProgramResult {
    cvlr_solana::token::spl_token_2022_transfer( vault_assets, user_assets, authority, amount)
}

pub fn spl_token_account_amount(info: &AccountInfo) -> Result<u64, ProgramError> {
    // We can return Err non-deterministically if needed.
    Ok(cvlr_solana::token::spl_token_account_get_amount(info))
}
