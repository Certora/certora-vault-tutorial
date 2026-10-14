use crate::utils::math::FeeBps;
use crate::utils::{guards::require_gt, math::mul_div_floor};
use crate::{VaultError, VaultResult};

// Projection of the on-chain `Vault` to the fields the verified core reads/writes.
// `PodU64` (little-endian `[u8; 8]`, only ever accessed through `u64::from`/`.into()`)
// is represented by its `u64` value.
#[cfg_attr(test, derive(Default))]
pub struct Vault {
    pub shares: u64,
    pub assets: u64,

    pub fee_bps: u64,
    pub fee_amount: u64,
}

impl Vault {
    pub fn num_shares(&self) -> u64 {
        self.shares
    }

    pub fn num_assets(&self) -> u64 {
        self.assets
    }

    pub fn fee_amount(&self) -> u64 {
        self.fee_amount
    }

    pub fn clear_fee_amount(&mut self) {
        self.fee_amount = 0u64;
    }

    pub fn fee_in_bps(&self) -> VaultResult<FeeBps> {
        self.fee_bps.try_into()
    }

    pub fn is_solvent(&self) -> bool {
        self.num_shares() <= self.num_assets()
    }

    pub fn check_invariant(&self) -> VaultResult<()> {
        // `bool::then_some(()).ok_or(e)` written as the equivalent `if`.
        if self.is_solvent() {
            Ok(())
        } else {
            Err(VaultError::GuardFail)
        }
    }

    pub fn convert_shares_to_assets(&self, shares: u64) -> VaultResult<u64> {
        let assets = if self.num_shares() == self.num_assets() {
            shares
        } else {
            mul_div_floor(shares, self.num_assets(), self.num_shares())?
        };
        Ok(assets)
    }

    pub fn convert_assets_to_shares(&self, token: u64) -> VaultResult<u64> {
        let shares = if self.num_shares() == self.num_assets() {
            token
        } else {
            mul_div_floor(token, self.num_shares(), self.num_assets())?
        };

        Ok(shares)
    }

    pub fn burn_shares(&mut self, amt: u64) -> VaultResult<()> {
        self.shares = self
            .num_shares()
            .checked_sub(amt)
            .ok_or(VaultError::MathOverflow)?;
        Ok(())
    }

    pub fn mint_shares(&mut self, amt: u64) -> VaultResult<()> {
        require_gt!(amt, 0, VaultError::GuardFail);
        self.shares = self
            .num_shares()
            .checked_add(amt)
            .ok_or(VaultError::MathOverflow)?;
        Ok(())
    }

    pub fn add_token(&mut self, amt: u64) -> VaultResult<()> {
        require_gt!(amt, 0, VaultError::GuardFail);
        self.assets = self
            .num_assets()
            .checked_add(amt)
            .ok_or(VaultError::MathOverflow)?;
        Ok(())
    }

    pub fn del_token(&mut self, amt: u64) -> VaultResult<()> {
        self.assets = self
            .num_assets()
            .checked_sub(amt)
            .ok_or(VaultError::MathOverflow)?;

        // ensure that vault is still solvent after slashing
        self.check_invariant()?;
        Ok(())
    }
}
