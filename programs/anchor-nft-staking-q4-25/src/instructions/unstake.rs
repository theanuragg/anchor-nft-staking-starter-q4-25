use anchor_lang::prelude::*;
use mpl_core::{instructions::RemovePluginV1CpiBuilder, types::PluginType, ID as CORE_PROGRAM_ID};

use crate::{
    errors::StakeError,
    state::{CollectionInfo, StakeAccount, StakeConfig, UserAccount},
};

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = asset.owner == &CORE_PROGRAM_ID @ StakeError::InvalidAsset,
        constraint = !asset.data_is_empty() @ StakeError::AssetNotInitialized
    )]
    /// CHECK: Verified by mpl-core
    pub asset: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = collection.owner == &CORE_PROGRAM_ID @ StakeError::InvalidCollection,
        constraint = !collection.data_is_empty() @ StakeError::CollectionNotInitialized
    )]
    /// CHECK: Verified by mpl-core
    pub collection: UncheckedAccount<'info>,

    #[account(
        seeds = [b"collection_info", collection.key().as_ref()],
        bump = collection_info.bump,
    )]
    pub collection_info: Account<'info, CollectionInfo>,

    #[account(
        mut,
        seeds = [b"user", user.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        mut,
        seeds = [b"stake", asset.key().as_ref()],
        bump = stake_account.bump,
        constraint = stake_account.owner == user.key() @ StakeError::NotOwner,
        close = user
    )]
    pub stake_account: Account<'info, StakeAccount>,

    #[account(
        seeds = [b"config"],
        bump = config.bump,
    )]
    pub config: Account<'info, StakeConfig>,

    #[account(address = CORE_PROGRAM_ID)]
    /// CHECK: Verified by address constraint
    pub core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Unstake<'info> {
    pub fn unstake(&mut self) -> Result<()> {
        let current_time = Clock::get()?.unix_timestamp;
        let time_staked = current_time - self.stake_account.staked_at;

        // Verify freeze period has passed
        require!(
            time_staked >= self.config.freeze_period as i64,
            StakeError::FreezePeriodNotPassed
        );

        // Calculate points earned (points_per_stake per second)
        let points_earned = (time_staked as u32)
            .checked_mul(self.config.points_per_stake as u32)
            .unwrap();

        // Update user account
        self.user_account.points = self.user_account.points.checked_add(points_earned).unwrap();
        self.user_account.amount_staked -= 1;

        // Remove freeze delegate plugin from the NFT
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"collection_info",
            &self.collection.key().to_bytes(),
            &[self.collection_info.bump],
        ]];

        RemovePluginV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.user.to_account_info())
            .authority(Some(&self.collection_info.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin_type(PluginType::FreezeDelegate)
            .invoke_signed(signer_seeds)?;

        Ok(())
    }
}
