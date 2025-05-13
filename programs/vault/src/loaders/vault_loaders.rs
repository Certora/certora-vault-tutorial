use std::{
    cell::{Ref, RefMut},
    mem::size_of,
};

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    state::{create_vault_assets_account_address, Vault},
    utils::guards::require_eq,
};

use super::{Signer, SplTokenProgramInfo};

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
        Self { info: info.clone() }.validate()
    }
}

impl<'info> VaultInfo<'info> {
    pub fn validate(self) -> Result<Self, ProgramError> {
        self.get()?.validate()?;
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

pub struct VaultAssetsAccount<'info> {
    pub info: AccountInfo<'info>,
}

impl<'info> VaultAssetsAccount<'info> {
    pub fn validate(self) -> Result<Self, ProgramError> {
        Ok(self)
    }

    pub fn check_vault(&self, vault_pk: &Pubkey, vault: &Vault) -> ProgramResult {
        let expected_pk = create_vault_assets_account_address(vault_pk, vault)?;
        require_eq!(self.info.key, &expected_pk, ProgramError::InvalidArgument);
        Ok(())
    }
}

impl<'info> AsRef<AccountInfo<'info>> for VaultAssetsAccount<'info> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.info
    }
}

impl<'info> TryFrom<&AccountInfo<'info>> for VaultAssetsAccount<'info> {
    type Error = ProgramError;
    fn try_from(info: &AccountInfo<'info>) -> Result<Self, Self::Error> {
        Self { info: info.clone() }.validate()
    }
}

pub struct DepositContext<'info> {
    // the vault
    pub vault_info: VaultInfo<'info>,
    // token account of the vault deposit
    pub vault_assets_account: VaultAssetsAccount<'info>,
    // mint for assets token
    pub assets_mint: AccountInfo<'info>,
    pub shares_mint: AccountInfo<'info>,
    // token account for the user making a deposit
    pub user_assets_account: AccountInfo<'info>,
    // signing authority for the user assets account
    pub authority: Signer<'info>,
    pub user_shares_account: AccountInfo<'info>,
    // SPL token program to make the transfer
    pub spl_token_program: SplTokenProgramInfo<'info>,
}

impl<'info> DepositContext<'info> {
    pub fn validate(self) -> Result<Self, ProgramError> {
        let vault = self.vault_info.get()?;
        require_eq!(
            &vault.assets_mint,
            self.assets_mint.key,
            ProgramError::InvalidArgument
        );

        require_eq!(
            &vault.shares_mint,
            self.shares_mint.key,
            ProgramError::InvalidArgument
        );

        require_eq!(
            &vault.vault_assets_account,
            self.vault_assets_account.as_ref().key,
            ProgramError::InvalidArgument
        );

        self.vault_assets_account
            .check_vault(self.vault_info.as_ref().key, &vault)?;

        drop(vault);
        Ok(self)
    }

    pub fn load(accounts: &[AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let iter = &mut accounts.iter();
        Self {
            vault_info: next_account_info(iter)?.try_into()?,
            vault_assets_account: next_account_info(iter)?.try_into()?,
            assets_mint: next_account_info(iter)?.clone(),
            shares_mint: next_account_info(iter)?.clone(),
            user_assets_account: next_account_info(iter)?.clone(),
            authority: next_account_info(iter)?.try_into()?,
            user_shares_account: next_account_info(iter)?.clone(),
            spl_token_program: next_account_info(iter)?.try_into()?,
        }
        .validate()
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
    pub spl_token_program: SplTokenProgramInfo<'info>,
}

impl<'info> RedeemSharesContext<'info> {
    pub fn validate(self) -> Result<Self, ProgramError> {
        let vault = self.vault_info.get()?;
        require_eq!(
            &vault.assets_mint,
            self.assets_mint.key,
            ProgramError::InvalidArgument
        );

        require_eq!(
            &vault.shares_mint,
            self.shares_mint.key,
            ProgramError::InvalidArgument
        );

        require_eq!(
            &vault.vault_assets_account,
            self.vault_assets_account.key,
            ProgramError::InvalidArgument
        );

        drop(vault);
        Ok(self)
    }
    pub fn load(accounts: &[AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let iter = &mut accounts.iter();
        Self {
            vault_info: next_account_info(iter)?.try_into()?,
            vault_assets_account: next_account_info(iter)?.clone(),
            assets_mint: next_account_info(iter)?.clone(),
            shares_mint: next_account_info(iter)?.clone(),
            user_shares_account: next_account_info(iter)?.clone(),
            authority: next_account_info(iter)?.try_into()?,
            user_assets_account: next_account_info(iter)?.clone(),
            spl_token_program: next_account_info(iter)?.try_into()?,
        }
        .validate()
    }
}

pub struct UpdateRewardContext<'info> {
    pub vault_info: VaultInfo<'info>,
    pub vault_assets_account: AccountInfo<'info>,
}

impl<'info> UpdateRewardContext<'info> {
    pub fn validate(self) -> Result<Self, ProgramError> {
        let vault = self.vault_info.get()?;
        require_eq!(
            &vault.vault_assets_account,
            self.vault_assets_account.key,
            ProgramError::InvalidArgument
        );

        drop(vault);
        Ok(self)
    }
}

impl<'info> UpdateRewardContext<'info> {
    pub fn load(accounts: &[AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let iter = &mut accounts.iter();
        Self {
            vault_info: next_account_info(iter)?.try_into()?,
            vault_assets_account: next_account_info(iter)?.clone(),
        }
        .validate()
    }
}
