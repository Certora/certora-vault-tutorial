use solana_program::program_error::ProgramError;

#[derive(Debug, PartialEq)]
pub enum VaultError {
    Unspecified,
    MathOverflow,
    GuardFail,
    SelfTransfer,
}

// Define a custom Result type
pub type VaultResult<T> = std::result::Result<T, VaultError>;

impl From<VaultError> for ProgramError {
    fn from(e: VaultError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
