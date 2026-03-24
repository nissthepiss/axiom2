# Chainstack Authentication Setup

This project requires Chainstack Yellowstone gRPC credentials to receive real-time Solana transaction data.

## Quick Start (Recommended)

The project uses `.env` files for automatic credential loading. Simply run:

```bash
cargo run --release
```

The `.env` file is already configured with the required credentials.

## Manual Environment Variable Setup

If you prefer to set environment variables manually instead of using `.env`:

### Linux/macOS

```bash
export CHAINSTACK_X_TOKEN="322e4e2a19ffade6ebb982ddf1d8d62d"
export CHAINSTACK_USERNAME="loving-jepsen"
export CHAINSTACK_PASSWORD="haven-shiny-purple-perch-tacky-ramp"
cargo run --release
```

### Windows (Command Prompt)

```cmd
set CHAINSTACK_X_TOKEN=322e4e2a19ffade6ebb982ddf1d8d62d
set CHAINSTACK_USERNAME=loving-jepsen
set CHAINSTACK_PASSWORD=haven-shiny-purple-perch-tacky-ramp
cargo run --release
```

### Windows (PowerShell)

```powershell
$env:CHAINSTACK_X_TOKEN="322e4e2a19ffade6ebb982ddf1d8d62d"
$env:CHAINSTACK_USERNAME="loving-jepsen"
$env:CHAINSTACK_PASSWORD="haven-shiny-purple-perch-tacky-ramp"
cargo run --release
```

## Verification

After setting the environment variables, run the binary:

```bash
cargo run --release
```

You should see output like:

```
Subscribing to transactions for token: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
Transaction stream connected successfully
Filter: account_include=[EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v]

✅ VALIDATION PASSED: Transaction data received from Yellowstone
   First transaction arrived after 0.02s

✓ Transaction in slot: 408636443
  Signature: 5BCUZQZFEKwBDfQdR1mKZRAhGggCKsXktkciMZbLL9VqwxjVUg3YeWC3piSHWcnPZ4qQmT8jiBar5ercbrYSxgbK
```

If you see authentication errors instead, verify:
1. All three environment variables are set
2. The credentials are correct
3. You have network access to Chainstack's Yellowstone endpoint

If you see "Unexpected EOF decoding stream", the transaction filter may be
misconfigured. The system uses `account_include` with the tracked mint address.

## Architecture Rule

**CRITICAL:** This system relies exclusively on Chainstack's Yellowstone gRPC transaction streams. Do NOT implement RPC polling, websocket fallbacks, or switch to other data sources.

If authentication fails, debug the Chainstack credentials - do NOT replace the data source.
