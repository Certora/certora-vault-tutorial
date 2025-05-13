use bytemuck::{Pod, Zeroable};
use solana_program::pubkey::{Pubkey, PubkeyError};

use crate::utils::math::FeeBps;
use crate::utils::{
    guards::{require_gt, require_ne},
    math::mul_div_floor,
};
use crate::{VaultError, VaultResult};
use spl_pod::primitives::PodU64;

#[repr(C)]
#[derive(Default, Pod, Copy, Clone, Zeroable)]
pub struct Vault {
    pub admin: Pubkey,
    pub shares_mint: Pubkey,
    pub assets_mint: Pubkey,

    pub shares: PodU64,
    pub assets: PodU64,

    pub fee_bps: PodU64,
    pub fee_amount: PodU64,
    pub fee_token_account: Pubkey,

    pub vault_assets_account: Pubkey,
    pub vault_assets_account_bump: u8,
}

impl Vault {
    pub fn new() -> Self {
        Vault::default()
    }

    pub fn num_shares(&self) -> u64 {
        self.shares.into()
    }

    pub fn num_assets(&self) -> u64 {
        self.assets.into()
    }

    pub fn fee_amount(&self) -> u64 {
        self.fee_amount.into()
    }

    pub fn clear_fee_amount(&mut self) {
        self.fee_amount = 0u64.into();
    }

    pub fn gross_assets(&self) -> u64 {
        // -- guaranteed to not overflow
        self.num_assets().checked_add(self.fee_amount()).unwrap()
    }

    pub fn fee_in_bps(&self) -> VaultResult<FeeBps> {
        u64::from(self.fee_bps).try_into()
    }

    pub fn is_solvent(&self) -> bool {
        self.num_shares() <= self.num_assets()
    }

    pub fn check_invariant(&self) -> VaultResult<()> {
        self.is_solvent().then_some(()).ok_or(VaultError::GuardFail)
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
            .ok_or(VaultError::MathOverflow)?
            .into();
        Ok(())
    }

    pub fn mint_shares(&mut self, amt: u64) -> VaultResult<()> {
        require_gt!(amt, 0, VaultError::GuardFail);
        self.shares = self
            .num_shares()
            .checked_add(amt)
            .ok_or(VaultError::MathOverflow)?
            .into();
        Ok(())
    }

    pub fn add_token(&mut self, amt: u64) -> VaultResult<()> {
        require_gt!(amt, 0, VaultError::GuardFail);
        self.assets = self
            .num_assets()
            .checked_add(amt)
            .ok_or(VaultError::MathOverflow)?
            .into();
        Ok(())
    }

    pub fn del_token(&mut self, amt: u64) -> VaultResult<()> {
        self.assets = self
            .num_assets()
            .checked_sub(amt)
            .ok_or(VaultError::MathOverflow)?
            .into();

        // ensure that vault is still solvent after slashing
        self.check_invariant()?;
        Ok(())
    }

    pub fn validate(&self) -> VaultResult<()> {
        require_ne!(self.assets_mint, self.shares_mint, VaultError::GuardFail);
        Ok(())
    }
}

/// Seeds for the PDA vault token account
#[macro_export]
macro_rules! vault_assets_account_seeds {
    ($vault_pk: expr) => {
        &[b"assets", vault_pk.as_ref()]
    };
}

/// Seeds for the PDA vault token account with seeds
#[macro_export]
macro_rules! vault_assets_account_seeds_with_bump {
    ( $vault_pk:expr, $bump:expr ) => {
        &[b"assets", $vault_pk.as_ref(), &[$bump]]
    };
}

pub fn create_vault_assets_account_address(
    vault_pk: &Pubkey,
    vault: &Vault,
) -> Result<Pubkey, PubkeyError> {
    Pubkey::create_program_address(
        vault_assets_account_seeds_with_bump!(vault_pk, vault.vault_assets_account_bump),
        &crate::ID,
    )
}
