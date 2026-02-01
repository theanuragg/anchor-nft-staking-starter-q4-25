use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, Mint, MintTo, Token, TokenAccount},
};

use crate::state::{StakeConfig, UserAccount};

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"user", user.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        seeds = [b"config"],
        bump = config.bump,
    )]
    pub config: Account<'info, StakeConfig>,

    #[account(
        mut,
        seeds = [b"rewards", config.key().as_ref()],
        bump = config.rewards_bump,
    )]
    pub reward_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = reward_mint,
        associated_token::authority = user,
    )]
    pub user_reward_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Claim<'info> {
    pub fn claim(&mut self) -> Result<()> {
        let points = self.user_account.points;

        // Ensure user has points to claim
        require!(points > 0, crate::errors::StakeError::NoPointsToClaim);

        // Mint reward tokens (1:1 ratio with points, accounting for decimals)
        let amount = (points as u64)
            .checked_mul(10u64.pow(self.reward_mint.decimals as u32))
            .unwrap();

        let signer_seeds: &[&[&[u8]]] = &[&[b"config", &[self.config.bump]]];

        mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintTo {
                    mint: self.reward_mint.to_account_info(),
                    to: self.user_reward_ata.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        // Reset user points
        self.user_account.points = 0;

        Ok(())
    }
}
