use num_enum::TryFromPrimitive;

#[repr(u8)]
#[derive(TryFromPrimitive, Debug, Copy, Clone, PartialEq, Eq)]
pub enum CertoraVaultInstruction {
    CreateVault = 0,
    Deposit = 1,
    DepositWithFee = 2,
}
