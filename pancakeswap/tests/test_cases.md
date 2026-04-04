# PancakeSwap — Test Cases

## TC-1: Quote BSC (Happy Path)
- Command: `./target/release/pancakeswap quote --from WBNB --to USDT --amount 0.001 --chain 56`
- Expected: Output contains USDT amount > 0, fee tier shown, QuoterV2 address shown
- Priority: P0
- Type: read

## TC-2: Pools BSC (Happy Path)
- Command: `./target/release/pancakeswap pools --token0 0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c --token1 0x55d398326f99059fF775485246999027B3197955 --chain 56`
- Expected: Lists at least 1 pool for WBNB/USDT, shows liquidity and fee tier
- Priority: P0
- Type: read

## TC-3: Quote Base (Happy Path)
- Command: `./target/release/pancakeswap quote --from WETH --to USDC --amount 0.001 --chain 8453`
- Expected: Output contains USDC amount > 0, rate ~2000+ USDC/WETH
- Priority: P0
- Type: read

## TC-4: Swap BSC Dry-run
- Command: `./target/release/pancakeswap swap --from WBNB --to USDT --amount 0.0001 --chain 56 --dry-run`
- Expected: Prints approve + swap calldata without submitting, exit code 0
- Priority: P0
- Type: dry-run

## TC-5: Swap Base Dry-run
- Command: `./target/release/pancakeswap swap --from WETH --to USDC --amount 0.00005 --chain 8453 --dry-run`
- Expected: Prints approve + swap calldata, fee=0.01%, recipient=wallet address, exit code 0
- Priority: P0
- Type: dry-run

## TC-6: Swap Base On-chain (P0)
- Command: `./target/release/pancakeswap swap --from WETH --to USDC --amount 0.00005 --chain 8453`
- Expected: txHash starting with 0x (66 chars), USDC balance increases ~0.10
- Priority: P0
- Type: on-chain
- Guardrail: max 0.00005 ETH (WETH) per tx

## TC-7: Unknown chain error
- Command: `./target/release/pancakeswap quote --from WETH --to USDC --amount 0.001 --chain 1`
- Expected: Error "Unsupported chain ID: 1", exit code 1
- Priority: P1
- Type: error-chain

## TC-8: Unknown token symbol error
- Command: `./target/release/pancakeswap quote --from FAKECOIN --to USDC --amount 0.001 --chain 8453`
- Expected: Error "Unknown token symbol", exit code 1
- Priority: P1
- Type: error-params
