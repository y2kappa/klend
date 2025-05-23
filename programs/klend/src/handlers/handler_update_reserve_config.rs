use anchor_lang::{prelude::*, Accounts};

use crate::{
    lending_market::lending_operations,
    state::{LendingMarket, Reserve, UpdateConfigMode},
    utils::Fraction,
    LendingError,
};

pub fn process(
    ctx: Context<UpdateReserveConfig>,
    mode: u64,
    value: &[u8],
    skip_validation: bool,
) -> Result<()> {
    let mode =
        UpdateConfigMode::try_from(mode).map_err(|_| ProgramError::InvalidInstructionData)?;

    let reserve = &mut ctx.accounts.reserve.load_mut()?;
    let market = ctx.accounts.lending_market.load()?;
    let name = reserve.config.token_info.symbol();

    msg!(
        "Updating reserve {:?} {} config with mode {:?}",
        ctx.accounts.reserve.key(),
        name,
        mode,
    );

    let clock = Clock::get()?;
    lending_operations::refresh_reserve(reserve, &clock, None, market.referral_fee_bps)?;

    lending_operations::update_reserve_config(reserve, mode, value);

    if skip_validation {
        let reserve_is_used = reserve.liquidity.available_amount
            > market.min_initial_deposit_amount
            || reserve.liquidity.total_borrow() > Fraction::ZERO
            || reserve.collateral.mint_total_supply > market.min_initial_deposit_amount;

        let reserve_blocks_deposits = reserve.config.deposit_limit == 0;
        let reserve_blocks_borrows = reserve.config.borrow_limit == 0;

        require!(
            !reserve_is_used && reserve_blocks_deposits && reserve_blocks_borrows,
            LendingError::InvalidConfig
        );
        msg!("WARNING! Skipping validation of the config");
    } else {
        lending_operations::utils::validate_reserve_config(
            &reserve.config,
            &market,
            ctx.accounts.reserve.key(),
        )?;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateReserveConfig<'info> {
    lending_market_owner: Signer<'info>,

    #[account(has_one = lending_market_owner)]
    lending_market: AccountLoader<'info, LendingMarket>,

    #[account(mut,
        has_one = lending_market
    )]
    reserve: AccountLoader<'info, Reserve>,
}
