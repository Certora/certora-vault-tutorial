use {
    crate::utils::guards::require,
    solana_program::{account_info::AccountInfo, program_error::ProgramError},
    std::result::Result,
};

pub struct Signer<'info> {
    pub info: AccountInfo<'info>,
}

impl<'info> TryFrom<&AccountInfo<'info>> for Signer<'info> {
    type Error = ProgramError;
    fn try_from(info: &AccountInfo<'info>) -> Result<Self, Self::Error> {
        require!(info.is_signer, ProgramError::MissingRequiredSignature);
        Ok(Self { info: info.clone() })
    }
}

impl<'info> AsRef<AccountInfo<'info>> for Signer<'info> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.info
    }
}

pub struct SplTokenProgramInfo<'info> {
    pub info: AccountInfo<'info>,
}

impl<'info> TryFrom<&AccountInfo<'info>> for SplTokenProgramInfo<'info> {
    type Error = ProgramError;
    fn try_from(info: &AccountInfo<'info>) -> Result<Self, Self::Error> {
        spl_token::check_program_account(info.key)?;
        Ok(Self { info: info.clone() })
    }
}

impl<'info> AsRef<AccountInfo<'info>> for SplTokenProgramInfo<'info> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.info
    }
}
