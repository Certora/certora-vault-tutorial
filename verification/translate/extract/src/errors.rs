#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum VaultError {
    Unspecified,
    MathOverflow,
    GuardFail,
    SelfTransfer,
}

// Define a custom Result type
pub type VaultResult<T> = std::result::Result<T, VaultError>;
