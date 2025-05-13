#[allow(unused_imports)]
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
};

#[cfg_attr(feature = "certora", 
    cvlr::mock_fn(with=crate::certora::mocks::processor::spl_transfer_assets_from_user))]
pub fn spl_transfer_assets_from_user(
    _amount: u64,
    _vault_assets: &AccountInfo,
    _user_assets: &AccountInfo,
    _mint: &AccountInfo,
    _authority: &AccountInfo,
    _spl_token_program: &AccountInfo,
) -> ProgramResult {
    // CPI call
    Ok(())
}

#[cfg_attr(feature = "certora", 
    cvlr::mock_fn(with=crate::certora::mocks::processor::spl_mint_shares))]
pub fn spl_mint_shares(
    _amount: u64,
    _user_shares_account: &AccountInfo,
    _mint: &AccountInfo,
    _authority: &AccountInfo,
    _spl_token_program: &AccountInfo,
) -> ProgramResult {
    // CPI call. Use PDA as a mint authority
    Ok(())
}

#[cfg_attr(feature = "certora", 
    cvlr::mock_fn(with=crate::certora::mocks::processor::spl_burn_shares))]
pub fn spl_burn_shares(
    _amount: u64,
    _user_shares_account: &AccountInfo,
    _mint: &AccountInfo,
    _authority: &AccountInfo,
    _spl_token_program: &AccountInfo,
) -> ProgramResult {
    // CPI call. Use PDA as a mint authority
    Ok(())
}

#[cfg_attr(feature = "certora", 
    cvlr::mock_fn(with=crate::certora::mocks::processor::spl_transfer_assets_from_vault))]
pub fn spl_transfer_assets_from_vault(
    _amount: u64,
    _vault_assets: &AccountInfo,
    _user_assets: &AccountInfo,
    _mint: &AccountInfo,
    _spl_token_program: &AccountInfo,
) -> ProgramResult {
    // Use PDA as vault assets owner
    Ok(())
}

#[cfg_attr(feature = "certora", cvlr::mock_fn(with=crate::certora::mocks::processor::spl_token_account_amount))]
pub fn spl_token_account_amount(_info: &AccountInfo) -> Result<u64, ProgramError> {
    // CPI call. Read amount value from the account
    Ok(0)
}
