use std::{
    cell::{Ref, RefMut},
    mem::size_of,
    result::Result,
};

use solana_program::{account_info::{next_account_info, AccountInfo}, program_error::ProgramError};

use crate::state::Vault;

pub struct VaultInfo<'info> {
    info: AccountInfo<'info>,
}

impl<'info> AsRef<AccountInfo<'info>> for VaultInfo<'info> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.info
    }
}

impl<'info> TryFrom<AccountInfo<'info>> for VaultInfo<'info> {
    type Error = ProgramError;

    fn try_from(info: AccountInfo<'info>) -> Result<Self, Self::Error> {
        // owned by vault program
        // has discriminant
        Self { info }.check_is_valid()
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
    pub vault_info: VaultInfo<'info>,
    pub vault_assets_account: AccountInfo<'info>,
    pub user_token_account: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub spl_token_program: AccountInfo<'info>,
}

impl<'info> DepositContext<'info> {
    pub fn load(accounts: &[AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let iter = &mut accounts.iter();
        Ok(Self {
            vault_info: next_account_info(iter)?.clone().try_into()?,
            vault_assets_account: next_account_info(iter)?.clone(),
            user_token_account: next_account_info(iter)?.clone(),
            authority: next_account_info(iter)?.clone(),
            spl_token_program: next_account_info(iter)?.clone(),
        })
    }
}
