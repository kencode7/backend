use anchor_lang::prelude::*;

declare_id!("4L6BwTs3J5deHpTLSHGPZKQKn9uhLFMKnKjhjqeobQ26");

#[program]
pub mod report_logger {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Report Logger initialized");
        Ok(())
    }

    pub fn log_report(ctx: Context<LogReport>, hash: [u8; 32]) -> Result<()> {
        let report = &mut ctx.accounts.report;
        report.authority = ctx.accounts.authority.key();
        report.hash = hash;
        report.timestamp = Clock::get()?.unix_timestamp;
        
        msg!("Report logged with hash: {:?}", hash);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}

#[derive(Accounts)]
pub struct LogReport<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 32 + 8
    )]
    pub report: Account<'info, Report>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Report {
    pub authority: Pubkey,    // 32 bytes
    pub hash: [u8; 32],       // 32 bytes
    pub timestamp: i64,       // 8 bytes
}
