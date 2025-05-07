use std::{
    cell::{Ref, RefMut},
    mem::size_of,
    result::Result,
};

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
};

use crate::{state::Vault, utils::guards::require};

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

pub struct VaultInfo<'info> {
    info: AccountInfo<'info>,
}

impl<'info> AsRef<AccountInfo<'info>> for VaultInfo<'info> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.info
    }
}

impl<'info> TryFrom<&AccountInfo<'info>> for VaultInfo<'info> {
    type Error = ProgramError;

    fn try_from(info: &AccountInfo<'info>) -> Result<Self, Self::Error> {
        // owned by vault program
        // has discriminant
        Self { info: info.clone() }.check_is_valid()
    }
}

impl<'info> VaultInfo<'info> {
    pub fn check_is_valid(self) -> Result<Self, ProgramError> {
        // require_neq!(self.assets_mint, self.shares_mint);

        Ok(self)
    }

    pub fn get(&self) -> Result<Ref<'_, Vault>, ProgramError> {
        let data = self.info.try_borrow_data()?;
        let res = Ref::map(data, |data| {
            bytemuck::from_bytes::<Vault>(&data[0..size_of::<Vault>()])
        });
        Ok(res)
    }

    pub fn get_mut(&self) -> Result<RefMut<'_, Vault>, ProgramError> {
        let data = self.info.try_borrow_mut_data()?;
        let res = RefMut::map(data, |data| {
            bytemuck::from_bytes_mut::<Vault>(&mut data[0..size_of::<Vault>()])
        });
        Ok(res)
    }
}

pub struct DepositContext<'info> {
    // the vault
    pub vault_info: VaultInfo<'info>,
    // token account of the vault deposit
    pub vault_assets_account: AccountInfo<'info>,
    // mint for assets token
    pub assets_mint: AccountInfo<'info>,
    pub shares_mint: AccountInfo<'info>,
    // token account for the user making a deposit
    pub user_assets_account: AccountInfo<'info>,
    // signing authority for the user assets account
    pub authority: Signer<'info>,
    pub user_shares_account: AccountInfo<'info>,
    // SPL token program to make the transfer
    pub spl_token_program: AccountInfo<'info>,
}

impl<'info> DepositContext<'info> {
    pub fn load(accounts: &[AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let iter = &mut accounts.iter();
        Ok(Self {
            vault_info: next_account_info(iter)?.try_into()?,
            vault_assets_account: next_account_info(iter)?.clone(),
            assets_mint: next_account_info(iter)?.clone(),
            shares_mint: next_account_info(iter)?.clone(),
            user_assets_account: next_account_info(iter)?.clone(),
            authority: next_account_info(iter)?.try_into()?,
            user_shares_account: next_account_info(iter)?.clone(),
            spl_token_program: next_account_info(iter)?.clone(),
        })
    }
}

pub struct RedeemSharesContext<'info> {
    pub vault_info: VaultInfo<'info>,
    pub vault_assets_account: AccountInfo<'info>,
    pub assets_mint: AccountInfo<'info>,
    pub shares_mint: AccountInfo<'info>,
    pub user_shares_account: AccountInfo<'info>,
    pub authority: Signer<'info>,
    pub user_assets_account: AccountInfo<'info>,
    pub spl_token_program: AccountInfo<'info>,
}

impl<'info> RedeemSharesContext<'info> {
    pub fn load(accounts: &[AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let iter = &mut accounts.iter();
        Ok(Self {
            vault_info: next_account_info(iter)?.try_into()?,
            vault_assets_account: next_account_info(iter)?.clone(),
            assets_mint: next_account_info(iter)?.clone(),
            shares_mint: next_account_info(iter)?.clone(),
            user_shares_account: next_account_info(iter)?.clone(),
            authority: next_account_info(iter)?.try_into()?,
            user_assets_account: next_account_info(iter)?.clone(),
            spl_token_program: next_account_info(iter)?.clone(),
        })
    }
}

pub struct UpdateRewardContext<'info> {
    pub vault_info: VaultInfo<'info>,
    pub vault_assets_account: AccountInfo<'info>,
}

impl<'info> UpdateRewardContext<'info> {
    pub fn load(accounts: &[AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let iter = &mut accounts.iter();
        Ok(Self {
            vault_info: next_account_info(iter)?.try_into()?,
            vault_assets_account: next_account_info(iter)?.clone(),
        })
    }
}