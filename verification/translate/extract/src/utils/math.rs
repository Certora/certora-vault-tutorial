use crate::{
    errors::{VaultError, VaultResult},
    utils::guards::require_le,
};

pub mod math_native {
    use super::*;

    pub fn mul_div_floor(a: u64, b: u64, c: u64) -> VaultResult<u64> {
        let q = (a as u128)
            .checked_mul(b as u128)
            .ok_or(VaultError::MathOverflow)?
            .checked_div(c as u128)
            .ok_or(VaultError::MathOverflow)?;
        // `u64::try_from(q).map_err(|_| VaultError::MathOverflow)` written as the equivalent
        // explicit range check (try_from fails exactly when q > u64::MAX).
        if q > u64::MAX as u128 {
            Err(VaultError::MathOverflow)
        } else {
            Ok(q as u64)
        }
    }
}

pub use math_native::*;

pub struct GrossAmount {
    pub net_amount: u64,
    pub fee: u64,
}

const ONE_IN_BPS: u64 = 10_000u64;
pub struct FeeBps(u64);

impl TryFrom<u64> for FeeBps {
    type Error = VaultError;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        require_le!(value, ONE_IN_BPS, VaultError::GuardFail);
        Ok(FeeBps(value))
    }
}

impl FeeBps {
    pub fn apply(&self, amt: u64) -> VaultResult<GrossAmount> {
        let fee = mul_div_floor(amt, self.0, ONE_IN_BPS)?;
        let net_amount = amt.checked_sub(fee).ok_or(VaultError::MathOverflow)?;
        Ok(GrossAmount { net_amount, fee })
    }
}
