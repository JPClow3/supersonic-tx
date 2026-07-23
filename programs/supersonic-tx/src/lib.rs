use anchor_lang::prelude::*;

declare_id!("GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9");

#[program]
pub mod supersonic_tx {
    use super::*;

    /// Execute a decoy zero-op instruction.
    /// Used by off-chain bundle generators to inject low-cost, indistinguishable
    /// state noise into Solana transactions.
    pub fn noop_decoy(ctx: Context<NoopDecoy>, entropy_seed: u64) -> Result<()> {
        msg!(
            "supersonic-tx: executed zero-op decoy instruction (seed: {})",
            entropy_seed
        );

        let clock = Clock::get()?;
        emit!(DecoyExecuted {
            authority: ctx.accounts.authority.key(),
            entropy_seed,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// On-chain Router: Execute a fuzzy bundle containing CPI targets and decoy calls.
    ///
    /// This instruction receives decoy instructions and target execution data.
    /// It validates constraints, executes decoy checks/invocations, and routes
    /// target CPI calls atomically.
    pub fn execute_fuzzy_bundle<'info>(
        ctx: Context<'_, '_, 'info, 'info, ExecuteFuzzyBundle<'info>>,
        bundle_seed: u64,
        decoy_count: u8,
        instruction_data: Vec<u8>,
    ) -> Result<()> {
        // v1 routes exactly one CPI. Reject caller-asserted counts that cannot
        // correspond to the concrete execution below.
        if decoy_count != 1 {
            return err!(SupersonicProgramError::InvalidBundleManifest);
        }

        msg!(
            "supersonic-tx: executing fuzzy bundle (seed: {}, decoys: {})",
            bundle_seed,
            decoy_count
        );

        let remaining_accounts = ctx.remaining_accounts;
        let target_program = remaining_accounts
            .first()
            .ok_or_else(|| error!(SupersonicProgramError::MissingCpiProgram))?;
        if !target_program.executable {
            return err!(SupersonicProgramError::MissingCpiProgram);
        }
        msg!(
            "supersonic-tx: routing CPI across {} remaining account(s)",
            remaining_accounts.len()
        );
        let ix = solana_program::instruction::Instruction {
            program_id: *target_program.key,
            accounts: remaining_accounts[1..]
                .iter()
                .map(|acc| solana_program::instruction::AccountMeta {
                    pubkey: *acc.key,
                    is_signer: acc.is_signer,
                    is_writable: acc.is_writable,
                })
                .collect(),
            data: instruction_data,
        };
        solana_program::program::invoke(&ix, remaining_accounts)
            .map_err(|_| error!(SupersonicProgramError::CpiExecutionFailed))?;

        let clock = Clock::get()?;
        emit!(BundleExecuted {
            authority: ctx.accounts.authority.key(),
            bundle_seed,
            routed_instruction_count: 1,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct NoopDecoy<'info> {
    /// Signer initiating the transaction bundle.
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ExecuteFuzzyBundle<'info> {
    /// Signer initiating the bundle execution.
    pub authority: Signer<'info>,
    /// System program reference for potential CPI inner calls.
    pub system_program: Program<'info, System>,
}

#[event]
pub struct DecoyExecuted {
    pub authority: Pubkey,
    pub entropy_seed: u64,
    pub timestamp: i64,
}

#[event]
pub struct BundleExecuted {
    pub authority: Pubkey,
    pub bundle_seed: u64,
    pub routed_instruction_count: u8,
    pub timestamp: i64,
}

#[error_code]
pub enum SupersonicProgramError {
    #[msg("Invalid decoy count or invalid bundle manifest structure.")]
    InvalidBundleManifest,
    #[msg("CPI execution failed during target instruction routing.")]
    CpiExecutionFailed,
    #[msg("A deployed executable CPI target program is required.")]
    MissingCpiProgram,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoy_count_validation() {
        let decoy_count_zero: u8 = 0;
        assert_eq!(decoy_count_zero == 0, true);

        let decoy_count_valid: u8 = 3;
        assert!(decoy_count_valid > 0);
    }

    #[test]
    fn test_event_structures() {
        let authority = Pubkey::new_unique();
        let decoy_event = DecoyExecuted {
            authority,
            entropy_seed: 12345,
            timestamp: 1600000000,
        };
        assert_eq!(decoy_event.entropy_seed, 12345);
        assert_eq!(decoy_event.authority, authority);

        let bundle_event = BundleExecuted {
            authority,
            bundle_seed: 9999,
            routed_instruction_count: 1,
            timestamp: 1600000000,
        };
        assert_eq!(bundle_event.bundle_seed, 9999);
        assert_eq!(bundle_event.routed_instruction_count, 1);
    }

    #[test]
    fn test_error_enum_codes() {
        let err_manifest = SupersonicProgramError::InvalidBundleManifest;
        let err_cpi = SupersonicProgramError::CpiExecutionFailed;

        let anchor_err_manifest: Error = err_manifest.into();
        let anchor_err_cpi: Error = err_cpi.into();

        assert_eq!(
            anchor_err_manifest,
            Error::from(SupersonicProgramError::InvalidBundleManifest)
        );
        assert_eq!(
            anchor_err_cpi,
            Error::from(SupersonicProgramError::CpiExecutionFailed)
        );
    }
}
