use std::collections::HashSet;

use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{mint_to, Mint, MintTo, Token, TokenAccount};
use arrayref::array_ref;
use solana_program::sysvar;

declare_id!("3az2EUU7xUaoEek3qRXdf9pkAZek764VJinWEWrGEn4h");

#[program]
pub mod charity_lottery {

    use anchor_spl::token;
    use solana_program::native_token::LAMPORTS_PER_SOL;

    use super::*;

    pub fn initialize_draw_number_recorder(
        ctx: Context<InitializeDrawNumberRecorder>,
    ) -> Result<()> {
        let recorder = &mut ctx.accounts.draw_number_recorder;

        let now_epoch = Clock::get()?.epoch;
        recorder.draw_number = 0;
        recorder.epoch = now_epoch;
        msg!("Draw number recorder created.");
        msg!("Draw number starts at: {}", recorder.draw_number);
        msg!("The epoch of charity lottery start at: {}", recorder.epoch);

        Ok(())
    }

    pub fn increase_draw_number(ctx: Context<IncreaseDrawNumber>) -> Result<()> {
        let recorder = &mut ctx.accounts.draw_number_recorder;
        let now_epoch = Clock::get()?.epoch;

        // need `now_epoch > recorder.epoch`
        // test `now_epoch + 1 > recorder.epoch`
        require!(
            now_epoch + 1 > recorder.epoch,
            CharityLotteryError::NotTimeYet
        );

        recorder.epoch = now_epoch;
        recorder.draw_number = recorder.draw_number.checked_add(1).unwrap();
        msg!(
            "The new draw is {} in {} epoch",
            recorder.draw_number,
            recorder.epoch
        );

        Ok(())
    }

    // New winnning numbers can be generated after increasing the draw number.
    pub fn new_winning_numbers(ctx: Context<NewWinningNumbers>) -> Result<()> {
        let recent_slot = Clock::get()?.slot;
        msg!("The recent slot is {}", recent_slot);

        let recent_slothashes = &ctx.accounts.recent_slothashes;
        let data = recent_slothashes.data.borrow();
        let most_recent = array_ref![data, 17, 32];
        msg!("The most recent slothash is {:?}", most_recent);

        let winning_numbers = &mut ctx.accounts.winning_numbers;
        let (common_numbers, special_number) = find_winning_numbers(most_recent);

        for (dest, &src) in winning_numbers
            .common_numbers
            .iter_mut()
            .zip(common_numbers.iter())
        {
            *dest = src;
        }
        winning_numbers.special_number = special_number;

        msg!("The winning numbers:");
        msg!("Common numbers: {:?}", winning_numbers.common_numbers);
        msg!("Special number: {}", winning_numbers.special_number);

        Ok(())
    }

    pub fn initialize_prize_pool(ctx: Context<InitializePrizePool>) -> Result<()> {
        msg!("Initialize the prize pool.");
        ctx.accounts.prize_pool.total_prize = 0;
        msg!(
            "The current total prize is {}",
            ctx.accounts.prize_pool.total_prize
        );
        Ok(())
    }

    pub fn initialize_charity_mint(_ctx: Context<InitializeCharityMint>) -> Result<()> {
        msg!("Token mint initialized");
        Ok(())
    }

    pub fn buy_lottery_ticket(
        ctx: Context<BuyLotteryTicket>,
        common_number: [u8; 5],
        special_number: u8,
    ) -> Result<()> {
        for num in common_number.iter() {
            require!(
                *num >= 1 && *num <= 64,
                CharityLotteryError::InvalidCommonNumber1
            )
        }
        require!(
            !(has_duplicates(&common_number)),
            CharityLotteryError::InvalidCommonNumber2
        );

        require!(
            special_number >= 1 && special_number <= 32,
            CharityLotteryError::InvalidSpecialNumber
        );
        let lottery_ticket = &mut ctx.accounts.lottery_ticket;
        lottery_ticket.common_numbers = common_number;
        lottery_ticket.special_number = special_number;
        lottery_ticket.is_checked = false;

        lottery_ticket.draw_number = ctx.accounts.draw_number_recorder.draw_number;

        let sol_guide_amount = (1u64).checked_mul(LAMPORTS_PER_SOL / 1000).unwrap();
        // transfer guide 1_000_000 lamports
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.buyer.to_account_info(),
                    to: ctx.accounts.guide.to_account_info(),
                },
                &[],
            ),
            sol_guide_amount,
        )?;

        let prize_token_amount = (2u64).checked_mul(LAMPORTS_PER_SOL / 1000).unwrap();
        // transfer prize pool 2_000_000 lamports
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.buyer.to_account_info(),
                    to: ctx.accounts.prize_pool.to_account_info(),
                },
                &[],
            ),
            prize_token_amount,
        )?;
        let prize_pool = &mut ctx.accounts.prize_pool;
        prize_pool.total_prize += (2u64).checked_mul(10u64.pow(6)).unwrap();

        let sol_mint_amount = (1u64).checked_mul(LAMPORTS_PER_SOL / 1000).unwrap();
        // transfer charity mint 1_000_000 lamports
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.buyer.to_account_info(),
                    to: ctx.accounts.charity_mint.to_account_info(),
                },
                &[],
            ),
            sol_mint_amount,
        )?;

        let mint_token_amount = (1u64)
            .checked_mul(10u64.pow(ctx.accounts.charity_mint.decimals as u32))
            .unwrap();
        // mint 1 charity token to the buyer
        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    authority: ctx.accounts.charity_mint.to_account_info(),
                    to: ctx.accounts.ass_token_account.to_account_info(),
                    mint: ctx.accounts.charity_mint.to_account_info(),
                },
                &[&["charity mint".as_bytes(), &[ctx.bumps.charity_mint]]],
            ),
            mint_token_amount,
        )?;

        msg!("You have bought a lottery ticket wiht the numbers: ");
        msg!("Common numbers {:?}", lottery_ticket.common_numbers);
        msg!("Special number {}", lottery_ticket.special_number);
        msg!("You have mint 1 charity token");

        Ok(())
    }

    pub fn check_ticket_numbers(ctx: Context<CheckTicketNumbers>, draw_number: u64) -> Result<()> {
        let lottery_ticket = &mut ctx.accounts.lottery_ticket;
        require!(
            lottery_ticket.draw_number == draw_number,
            CharityLotteryError::InvalidDrawNumber
        );

        require!(
            lottery_ticket.is_checked == false,
            CharityLotteryError::AlreadyChecked
        );

        let win_common_amounts = count_shared_elements(
            &lottery_ticket.common_numbers,
            &ctx.accounts.winning_numbers.common_numbers,
        );
        let is_win_special =
            lottery_ticket.special_number == ctx.accounts.winning_numbers.special_number;

        let count = get_winning_count(&win_common_amounts, &is_win_special);

        if count == 0 {
            msg!("It's a pity that you didn't win the prize.")
        } else if count == u64::MAX {
            msg!("Congratulation! You win the full prize pool.");

            // winning_pool transfer the full prize pool to the winner
            transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.system_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.prize_pool.to_account_info(),
                        to: ctx.accounts.buyer.to_account_info(),
                    },
                    &[&["prize pool".as_bytes(), &[ctx.bumps.prize_pool]]],
                ),
                ctx.accounts.prize_pool.total_prize,
            )?;
            let prize_pool = &mut ctx.accounts.prize_pool;
            prize_pool.total_prize = 0;
        } else {
            let full_prize_sol_amount = count.checked_mul(10u64.pow(6)).unwrap();
            msg!(
                "Congratulation! You win {} lamports.",
                full_prize_sol_amount
            );

            // winning_pool transfer the full prize pool to the winner
            transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.system_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.prize_pool.to_account_info(),
                        to: ctx.accounts.buyer.to_account_info(),
                    },
                    &[&["prize pool".as_bytes(), &[ctx.bumps.prize_pool]]],
                ),
                full_prize_sol_amount,
            )?;
            let prize_pool = &mut ctx.accounts.prize_pool;
            prize_pool.total_prize -= full_prize_sol_amount;
        }

        lottery_ticket.is_checked = true;

        Ok(())
    }

    pub fn publish_charity_project(
        ctx: Context<PublishCharityProject>,
        project_name: String,
        description: String,
    ) -> Result<()> {
        msg!("Charity project account created.");
        msg!("Project name: {}", project_name);
        msg!("Description: {}", description);

        let charity_project = &mut ctx.accounts.charity_project;
        charity_project.creator = ctx.accounts.creator.key();
        charity_project.project_name = project_name;
        charity_project.description = description;

        Ok(())
    }

    pub fn support_charity_project(ctx: Context<SupportCharityProject>, amount: u64) -> Result<()> {
        msg!(
            "Transfer {} charity tokens to charity project",
            amount * 10 ^ 6
        );

        let support_token_amount = amount.checked_mul(10u64.pow(6)).unwrap();
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.supporter_asstoken_account.to_account_info(),
                    to: ctx.accounts.project_asstoken_account.to_account_info(),
                    authority: ctx.accounts.supporter.to_account_info(),
                },
                &[],
            ),
            support_token_amount,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeDrawNumberRecorder<'info> {
    #[account(
        init,
        seeds = ["draw number".as_bytes()],
        bump,
        payer = initializer,
        space = 8 + 8 + 8
    )]
    pub draw_number_recorder: Account<'info, DrawNumberRecorder>,
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct IncreaseDrawNumber<'info> {
    #[account(
        mut,
        seeds = ["draw number".as_bytes()],
        bump,
    )]
    pub draw_number_recorder: Account<'info, DrawNumberRecorder>,
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct NewWinningNumbers<'info> {
    #[account(
        init,
        seeds = ["winning numbers".as_bytes(), &draw_number_recorder.draw_number.to_le_bytes()],
        bump,
        payer = initializer,
        space = 8 + 5 + 1
    )]
    pub winning_numbers: Account<'info, WinningNumbers>,
    #[account(
        seeds = ["draw number".as_bytes()],
        bump,
    )]
    pub draw_number_recorder: Account<'info, DrawNumberRecorder>,
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: Explicit wrapper for AccountInfo type to emphasize that no checks are performed
    #[account(address = sysvar::slot_hashes::id())]
    pub recent_slothashes: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct InitializePrizePool<'info> {
    #[account(
        init,
        seeds = ["prize pool".as_bytes()],
        bump,
        payer = initializer,
        space = 8 + 8
    )]
    pub prize_pool: Account<'info, PrizePool>,
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeCharityMint<'info> {
    #[account(
        init,
        seeds = ["charity mint".as_bytes()],
        bump,
        payer = initializer,
        mint::authority = charity_mint,
        mint::decimals = 6,
    )]
    pub charity_mint: Account<'info, Mint>,
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BuyLotteryTicket<'info> {
    #[account(
        init_if_needed,
        seeds = ["buy lottery ticket".as_bytes(), buyer.key().as_ref()],
        bump,
        payer = buyer,
        space = 8 + 8 + 1 + 5 + 1
    )]
    pub lottery_ticket: Account<'info, LotteryTicket>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    // The person who guides someone to buy lottery ticket.
    /// CHECK: Explicit wrapper for AccountInfo type to emphasize that no checks are performed
    #[account(mut)]
    pub guide: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    #[account(
        seeds = ["prize pool".as_bytes()],
        bump,
        mut,
    )]
    pub prize_pool: Account<'info, PrizePool>,
    #[account(
        seeds=["charity mint".as_bytes()],
        bump,
        mut
    )]
    pub charity_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = charity_mint,
        associated_token::authority = buyer
    )]
    pub ass_token_account: Account<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    #[account(
        seeds = ["draw number".as_bytes()],
        bump,
    )]
    pub draw_number_recorder: Account<'info, DrawNumberRecorder>,
}

#[derive(Accounts)]
#[instruction(draw_number: u64)]
pub struct CheckTicketNumbers<'info> {
    #[account(
        seeds = ["buy lottery ticket".as_bytes(), buyer.key().as_ref()],
        bump,
    )]
    pub lottery_ticket: Account<'info, LotteryTicket>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(
        seeds = ["winning numbers".as_bytes(), &draw_number.to_le_bytes()],
        bump,
    )]
    pub winning_numbers: Account<'info, WinningNumbers>,
    #[account(
        seeds = ["prize pool".as_bytes()],
        bump,
        mut,
    )]
    pub prize_pool: Account<'info, PrizePool>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(project_name: String, description: String)]
pub struct PublishCharityProject<'info> {
    #[account(
        init,
        seeds = [project_name.as_bytes(), creator.key().as_ref()],
        bump,
        payer = creator,
        space = 8 + 32 + 4 + project_name.len() + 4 + description.len()
    )]
    pub charity_project: Account<'info, CharityProject>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(
        init_if_needed,
        payer = creator,
        associated_token::mint = charity_mint,
        associated_token::authority = creator
    )]
    pub project_asstoken_account: Account<'info, TokenAccount>,
    #[account(
        seeds=["charity mint".as_bytes()],
        bump,
        mut
    )]
    pub charity_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct SupportCharityProject<'info> {
    #[account(mut)]
    pub supporter: Signer<'info>,
    #[account(
        associated_token::mint = charity_mint,
        associated_token::authority = supporter,
        mut
    )]
    pub supporter_asstoken_account: Account<'info, TokenAccount>,
    #[account(
        associated_token::mint = charity_mint,
        associated_token::authority = project_creator_account,
        mut
    )]
    pub project_asstoken_account: Account<'info, TokenAccount>,
    /// CHECK: The account which is transfered to.
    pub project_creator_account: UncheckedAccount<'info>,
    #[account(
        seeds=["charity mint".as_bytes()],
        bump
    )]
    pub charity_mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct DrawNumberRecorder {
    pub draw_number: u64, // 8
    pub epoch: u64,       // 8
}

#[account]
pub struct WinningNumbers {
    pub common_numbers: [u8; 5], // 1 * 5
    pub special_number: u8,      // 1
}

#[account]
pub struct PrizePool {
    pub total_prize: u64, // 8
}

#[account]
pub struct LotteryTicket {
    pub draw_number: u64,        // 8
    pub is_checked: bool,        // 1
    pub common_numbers: [u8; 5], // 1 * 5
    pub special_number: u8,      // 1
}

#[account]
pub struct CharityProject {
    pub creator: Pubkey,      // 32
    pub project_name: String, // 4 + len()
    pub description: String,  // 4 + len()
}

#[error_code]
enum CharityLotteryError {
    #[msg("It's not time yet.")]
    NotTimeYet,
    #[msg("There is a common number out of range.")]
    InvalidCommonNumber1,
    #[msg("There are duplicates in the common numbers.")]
    InvalidCommonNumber2,
    #[msg("There is something wrong with your special numbers.")]
    InvalidSpecialNumber,
    #[msg("The draw number of the lottery ticket is not matched.")]
    InvalidDrawNumber,
    #[msg("The lottery ticket has already been checked.")]
    AlreadyChecked,
}

fn find_winning_numbers(data: &[u8; 32]) -> ([u8; 5], u8) {
    let mut common_numbers = Vec::new();
    let mut special_number: u8 = 0;
    let mut seen_numbers = HashSet::new();

    let mut iteration: usize = 0;
    for &num in data.iter() {
        let num_mod64 = num % 64;

        if !seen_numbers.contains(&num_mod64) && num_mod64 != 0 {
            common_numbers.push(num_mod64);
            seen_numbers.insert(num_mod64);

            // find 5 common numbers
            if common_numbers.len() == 5 {
                iteration += 1;
                break;
            }
        }

        iteration += 1;
    }

    let common_numbers_array = match common_numbers.as_slice() {
        [a, b, c, d, e] => [*a, *b, *c, *d, *e],
        _ => [0, 0, 0, 0, 0],
    };

    for i in iteration..32 {
        if data[i] % 32 != 0 {
            special_number += data[i] % 32;
            break;
        }
    }

    (common_numbers_array, special_number)
}

fn has_duplicates(arr: &[u8; 5]) -> bool {
    let mut set = HashSet::new();

    for &num in arr.iter() {
        if !set.insert(num) {
            return true;
        }
    }
    return false;
}

fn count_shared_elements(arr1: &[u8; 5], arr2: &[u8; 5]) -> usize {
    let set1: HashSet<_> = arr1.iter().cloned().collect();
    let set2: HashSet<_> = arr2.iter().cloned().collect();

    set1.intersection(&set2).count()
}

fn get_winning_count(common: &usize, special: &bool) -> u64 {
    match (common, special) {
        (0, true) => 8,
        (1, true) => 8,
        (2, true) => 32,
        (3, false) => 32,
        (3, true) => 200,
        (4, false) => 200,
        (4, true) => 100_000,
        (5, false) => 200_000,
        (5, true) => u64::MAX,
        _ => 0,
    }
}
