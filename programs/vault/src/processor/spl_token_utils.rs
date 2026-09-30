use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
};

pub fn spl_transfer_assets_from_user<'a>(
    amount: u64,
    vault_assets: &AccountInfo<'a>,
    user_assets: &AccountInfo<'a>,
    _mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    spl_token_program: &AccountInfo<'a>,
) -> ProgramResult {
    let ix = spl_token::instruction::transfer(
        spl_token_program.key,
        user_assets.key,
        vault_assets.key,
        authority.key,
        &[],
        amount,
    )?;
    invoke(
        &ix,
        &[
            user_assets.clone(),
            vault_assets.clone(),
            authority.clone(),
            spl_token_program.clone(),
        ],
    )
}

/// The shares mint is its own mint authority, so `mint_signer_seeds` are the
/// shares mint PDA's seeds.
pub fn spl_mint_shares<'a>(
    amount: u64,
    user_shares_account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    spl_token_program: &AccountInfo<'a>,
    mint_signer_seeds: &[&[u8]],
) -> ProgramResult {
    let ix = spl_token::instruction::mint_to(
        spl_token_program.key,
        mint.key,
        user_shares_account.key,
        mint.key,
        &[],
        amount,
    )?;
    invoke_signed(
        &ix,
        &[
            mint.clone(),
            user_shares_account.clone(),
            spl_token_program.clone(),
        ],
        &[mint_signer_seeds],
    )
}

pub fn spl_burn_shares<'a>(
    amount: u64,
    user_shares_account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    spl_token_program: &AccountInfo<'a>,
) -> ProgramResult {
    let ix = spl_token::instruction::burn(
        spl_token_program.key,
        user_shares_account.key,
        mint.key,
        authority.key,
        &[],
        amount,
    )?;
    invoke(
        &ix,
        &[
            user_shares_account.clone(),
            mint.clone(),
            authority.clone(),
            spl_token_program.clone(),
        ],
    )
}

/// The vault assets account is its own owner, so `vault_assets_signer_seeds`
/// are the vault assets PDA's seeds.
pub fn spl_transfer_assets_from_vault<'a>(
    amount: u64,
    vault_assets: &AccountInfo<'a>,
    user_assets: &AccountInfo<'a>,
    _mint: &AccountInfo<'a>,
    spl_token_program: &AccountInfo<'a>,
    vault_assets_signer_seeds: &[&[u8]],
) -> ProgramResult {
    let ix = spl_token::instruction::transfer(
        spl_token_program.key,
        vault_assets.key,
        user_assets.key,
        vault_assets.key,
        &[],
        amount,
    )?;
    invoke_signed(
        &ix,
        &[
            vault_assets.clone(),
            user_assets.clone(),
            spl_token_program.clone(),
        ],
        &[vault_assets_signer_seeds],
    )
}

pub fn spl_token_account_amount(info: &AccountInfo) -> Result<u64, ProgramError> {
    let account = spl_token::state::Account::unpack(&info.try_borrow_data()?)?;
    Ok(account.amount)
}
