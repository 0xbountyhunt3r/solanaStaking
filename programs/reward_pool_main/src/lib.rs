use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token::{self, Mint, Token, TokenAccount, Transfer}};
use solana_program::pubkey::{self, Pubkey};

declare_id!("5cKMnczybrfdiZdtajESXuQrmJSd4xFquF6KFinNMBi3");

#[program]
pub mod reward_pool_main {
    use super::*;
    pub fn initialize(ctx: Context<InitializePool>) -> Result<()> {
        let reward_pool: &mut Account<RewardPoolState> = &mut ctx.accounts.reward_pool;
        reward_pool.tax_recipient = ctx.accounts.user.key();
        reward_pool.creator = ctx.accounts.user.key();
        reward_pool.token_mint = ctx.accounts.pool_token_mint.key();
        reward_pool.bump = ctx.bumps.reward_pool;
        Ok(())
    }

    #[allow(unused_variables)]
    pub fn deposit_reward(
        ctx: Context<DepositReward>, 
        token_address: Pubkey, 
        campaign_amount: u64,
        fee_amount: u64,
        campaign_id: u64
    ) -> Result<()> {
        // Logic for depositing rewards
        let reward_info = &mut ctx.accounts.reward_info;
        let reward_pool = &mut ctx.accounts.reward_pool;

        // Perform the token transfer for the campaign amount to the campaign's token account
        msg!("before transfer");
        let transfer_campaign_ix = Transfer {
            from: ctx.accounts.depositer_token_account.to_account_info(),
            to: ctx.accounts.campaign_token_account.to_account_info(),
            authority: ctx.accounts.depositer.to_account_info(),
        };
        msg!("{:?}",ctx.accounts.depositer.to_account_info());
        msg!("{:?}",ctx.accounts.depositer.to_account_info());
        msg!("before transfer");
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                transfer_campaign_ix
            ),
            campaign_amount,
        )?;

        // Initialize or update reward_info
        reward_info.token_address = token_address;
        reward_info.amount += campaign_amount; // Assuming accumulation of amounts if multiple deposits
        reward_info.owner_address = *ctx.accounts.depositer.key;
        reward_info.bump = ctx.bumps.reward_info;

        Ok(())
    }

    #[allow(unused_variables)]
    pub fn withdraw_reward(
        ctx: Context<WithdrawReward>, 
        campaign_id: u64, 
        amount: u64
    ) -> Result<()> {
        // Logic for withdrawing rewards
        let reward_info = &mut ctx.accounts.reward_info;
    

        if ctx.accounts.reward_pool.paused {
            return Err(ErrorCode::ProgramPaused.into());
        }

        if reward_info.amount < amount {
            return Err(ErrorCode::NotEnoughReward.into());
        }

        // Only campaign creator allowed to withdraw
        if *ctx.accounts.user.key != reward_info.owner_address {
            return Err(ErrorCode::OnlyCampaignCreatorAllowed.into());
        }

        reward_info.amount -= amount;
       
        // Perform the token transfer from the campaign account to the user token account
        // * use the same seeds that are used for the poolPDA
        let seeds = &[
        b"reward_pool".as_ref(),
        ctx.accounts.reward_pool.creator.as_ref(),
        &[ctx.accounts.reward_pool.bump]
        ];

        let transfer_reward_ix = Transfer {
            from: ctx.accounts.campaign_token_account.to_account_info(),
            to: ctx.accounts.user_vault.to_account_info(), 
            authority: ctx.accounts.reward_pool.to_account_info()
        };
        msg!("withdraw, before transfer");
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                transfer_reward_ix,
                &[&seeds[..]]
            ),
            amount,
        )?;

        Ok(())
    }


#[derive(Accounts)]
pub struct InitializePool<'info> {   
    // * now it is a PDA with the user.key() acting as the creator.
    #[account(
        init, 
        payer = user, 
        space = 8 + 32 + 32 + 32 + 8 +1,
        seeds=[b"reward_pool".as_ref(), user.key().as_ref()],
        bump)
    ]
    pub reward_pool: Account<'info, RewardPoolState>,
    
    pub pool_token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = user, 
        associated_token::mint = pool_token_mint,
        associated_token::authority = reward_pool
    )]
    pub pool_token_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}




#[derive(Accounts)]
#[instruction(campaign_id: u64)]
pub struct DepositReward<'info> {
    pub pool_token_mint: Account<'info, Mint>,

    #[account(
        mut, 
        seeds=[b"reward_pool".as_ref(), reward_pool.creator.as_ref()],
        bump = reward_pool.bump)
    ]
    pub reward_pool: Account<'info, RewardPoolState>,

    #[account(
        init_if_needed,
        payer = depositer,
        associated_token::mint = pool_token_mint, 
        associated_token::authority = depositer)
      ]
    pub depositer_token_account: Account<'info, TokenAccount>,
    
    // * the pool token account to hold the coins deposited
    #[account(
        mut,
        associated_token::mint = pool_token_mint, 
        associated_token::authority = reward_pool)
      ]
    pub campaign_token_account: Account<'info, TokenAccount>, // Token account to store the campaign's funds
    
   //Token program
    #[account(
        init,
        payer = depositer,
        // seeds = [b"reward_info", campaign_id.to_le_bytes().as_ref()],
        seeds=[b"reward_info".as_ref(), depositer.key().as_ref()],
        bump,
        space = 8 + 32 + 32 + 8 + 1 // @audit => space change. Assuming space for u64 (amount), 2 * Pubkey (token_address, owner_address), plus discriminator
    )]
    pub reward_info: Account<'info,RewardInfo>,

    #[account(mut)]
    pub depositer: Signer<'info>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,  // Assumes reward_info is initialized here if not already existing
    pub system_program: Program<'info, System>,
}



#[derive(Accounts)]
#[instruction(campaign_id: u64)]
pub struct WithdrawReward<'info> {
    pub pool_token_mint: Account<'info, Mint>,
    #[account(
        mut, 
        seeds=[b"reward_pool".as_ref(), reward_pool.creator.as_ref()],
        bump = reward_pool.bump)
    ]
    pub reward_pool: Account<'info, RewardPoolState>,

    #[account(
        mut,
        associated_token::mint = pool_token_mint, 
        associated_token::authority = user.key())
      ]
    pub user_vault: Account<'info, TokenAccount>, 

    #[account(
        mut,
        associated_token::mint = pool_token_mint, 
        associated_token::authority = reward_pool)
      ]
    pub campaign_token_account: Account<'info, TokenAccount>, // Token account to store the campaign's funds
     // Token program
    #[account(
        mut,
        seeds = [b"reward_info", user.key().as_ref()],
        bump = reward_info.bump
    )]
    pub reward_info: Account<'info, RewardInfo>,

    pub token_program: Program<'info, Token>, 
    pub system_program: Program<'info, System>,
    pub user: Signer<'info>
}


#[account]
pub struct RewardPoolState {
    pub creator: Pubkey,
    pub token_mint:Pubkey,
    pub tax_recipient: Pubkey,
    pub paused: bool,
    pub bump: u8,
}

#[account]
pub struct RewardInfo {
    pub token_address: Pubkey,
    pub owner_address: Pubkey,
    pub amount: u64,
    pub bump:u8,
}


}
#[error_code]
pub enum ErrorCode {
    #[msg("The campaign already exists.")]
    CampaignAlreadyExists,
    #[msg("Not enough reward in the pool.")]
    NotEnoughReward,
    #[msg("Claim amount exceeds allowed balance")]
    ClaimAmountExceedsAllowedBalance,
    #[msg("Reward already claimed")]
    RewardAlreadyClaimed,
    #[msg("Only campaign creator allowed to withdraw")]
    OnlyCampaignCreatorAllowed,
    #[msg("Invalid signer address")]
    InvalidSignerAddress,
    #[msg("Invalid owner address")]
    InvalidOwnerAddress,
    #[msg("Program is paused")]
    ProgramPaused,
    #[msg("Unauthorized")]
    Unauthorized,
}