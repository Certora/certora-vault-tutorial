use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
};

use crate::utils::guards::require_eq;

use super::{Signer, SplTokenProgramInfo, VaultInfo};

pub struct DepositWithFeeContext<'info> {
    // the vault
    pub vault_info: VaultInfo<'info>,
    // token account of the vault deposit
    pub vault_assets_account: AccountInfo<'info>,
    // fee token account (in asset tokens)
    pub vault_fee_account: AccountInfo<'info>,
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

impl<'info> DepositWithFeeContext<'info> {
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

        require_eq!(
            &vault.fee_token_account,
            self.vault_fee_account.key,
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
            vault_fee_account: next_account_info(iter)?.clone(),
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

pub struct CollectFeeContext<'info> {
    pub vault_info: VaultInfo<'info>,
    pub vault_assets_account: AccountInfo<'info>,
    pub assets_mint: AccountInfo<'info>,
    pub fee_collect_account: AccountInfo<'info>,
    pub authority: Signer<'info>,
    pub spl_token_program: SplTokenProgramInfo<'info>,
}

impl<'info> CollectFeeContext<'info> {
    pub fn validate(self) -> Result<Self, ProgramError> {
        let vault = self.vault_info.get()?;
        require_eq!(
            &vault.assets_mint,
            self.assets_mint.key,
            ProgramError::InvalidArgument
        );

        require_eq!(
            &vault.vault_assets_account,
            self.vault_assets_account.key,
            ProgramError::InvalidArgument
        );

        require_eq!(
            &vault.admin,
            self.authority.as_ref().key,
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
            fee_collect_account: next_account_info(iter)?.clone(),
            authority: next_account_info(iter)?.try_into()?,
            spl_token_program: next_account_info(iter)?.try_into()?,
        }
        .validate()
    }
}
