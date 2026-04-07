// Sanctum Validator LSTs — configuration and LST registry

/// Solana mainnet chain ID
pub const SOLANA_CHAIN_ID: u64 = 501;

/// Sanctum Router API base URL
pub const ROUTER_API_BASE: &str = "https://sanctum-s-api.fly.dev";

/// Sanctum Extra API base URL
pub const EXTRA_API_BASE: &str = "https://extra-api.sanctum.so";

/// Solana mainnet RPC
pub const SOLANA_RPC: &str = "https://api.mainnet-beta.solana.com";

/// SPL Stake Pool Program ID
pub const STAKE_POOL_PROGRAM: &str = "SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy";

/// Sanctum S Controller / SPool program (used as --to for swap)
pub const SPOOL_PROGRAM: &str = "5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx";

/// Token Program
pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

/// Associated Token Program
#[allow(dead_code)]
pub const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe1bx8";

/// System Program
pub const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";

/// Lamports per SOL
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

/// LST token decimals (all Sanctum LSTs: 9)
pub const LST_DECIMALS: u32 = 9;

/// wSOL mint (native SOL representation in swaps)
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

// ──────────────────────── LST Registry ────────────────────────

/// Pool program type — used to decide stake instruction path
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolProgram {
    /// Standard SPL Stake Pool (`SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy`)
    SplStakePool,
    /// Marinade custom program — not supported for `stake` in this plugin
    Marinade,
    /// SanctumSpl / SanctumSplMulti — uses SPL Stake Pool program variant
    SanctumSpl,
    /// Sanctum Infinity pool — handled by sanctum-infinity plugin
    Infinity,
    /// Wrapped SOL — not an LST
    WrappedSol,
}

pub struct LstConfig {
    pub symbol: &'static str,
    pub mint: &'static str,
    pub pool_program: PoolProgram,
    /// Stake pool account address. Empty string means fetch from RPC at runtime.
    pub stake_pool: &'static str,
}

pub const LSTS: &[LstConfig] = &[
    LstConfig {
        symbol: "jitoSOL",
        mint: "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn",
        pool_program: PoolProgram::SplStakePool,
        stake_pool: "Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb",
    },
    LstConfig {
        symbol: "mSOL",
        mint: "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So",
        pool_program: PoolProgram::Marinade,
        stake_pool: "", // Marinade uses custom program — not stakeable here
    },
    LstConfig {
        symbol: "jupSOL",
        mint: "jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v",
        pool_program: PoolProgram::SanctumSpl,
        stake_pool: "", // fetch from RPC at runtime
    },
    LstConfig {
        symbol: "bSOL",
        mint: "bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1",
        pool_program: PoolProgram::SplStakePool,
        stake_pool: "", // fetch from RPC at runtime
    },
    LstConfig {
        symbol: "compassSOL",
        mint: "Comp4ssDzXcLev2MnLuGNNFC4cmLPMng8qWHPvzAMU1h",
        pool_program: PoolProgram::SanctumSpl,
        stake_pool: "", // fetch from RPC at runtime
    },
    LstConfig {
        symbol: "hubSOL",
        mint: "HUBsveNpjo5pWqNkH57QzxjQASdTVXcSK7bVKTSZtcSX",
        pool_program: PoolProgram::SanctumSpl,
        stake_pool: "", // fetch from RPC at runtime
    },
    LstConfig {
        symbol: "bonkSOL",
        mint: "BonK1YhkXEGLZzwtcvRTip3gAL9nCeQD7ppZBLXhtTs",
        pool_program: PoolProgram::SanctumSpl,
        stake_pool: "", // fetch from RPC at runtime
    },
    LstConfig {
        symbol: "stakeSOL",
        mint: "st8QujHLPsX3d6HG9uQg9kJ91jFxUgruwsb1hyYXSNd",
        pool_program: PoolProgram::SanctumSpl,
        stake_pool: "", // fetch from RPC at runtime
    },
    LstConfig {
        symbol: "INF",
        mint: "5oVNBeEEQvYi1cX3ir8Dx5n1P7pdxydbGF2X4TxVusJm",
        pool_program: PoolProgram::Infinity,
        stake_pool: "", // covered by sanctum-infinity plugin
    },
    LstConfig {
        symbol: "wSOL",
        mint: "So11111111111111111111111111111111111111112",
        pool_program: PoolProgram::WrappedSol,
        stake_pool: "",
    },
];

/// Find LST config by symbol (case-insensitive) or exact mint address.
pub fn find_lst(input: &str) -> Option<&'static LstConfig> {
    let lower = input.to_lowercase();
    LSTS.iter().find(|l| {
        l.symbol.to_lowercase() == lower || l.mint == input
    })
}

/// Return mint address for symbol or passthrough if already a mint address.
pub fn resolve_mint(input: &str) -> &str {
    if let Some(lst) = find_lst(input) {
        lst.mint
    } else {
        input // assume it's already a mint address
    }
}
