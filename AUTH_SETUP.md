# Chainstack Authentication Setup

This project requires Chainstack Yellowstone gRPC credentials to receive real-time Solana transaction data.

## Required Environment Variables

Set the following environment variables before running the binary:

```bash
export CHAINSTACK_X_TOKEN="322e4e2a19ffade6ebb982ddf1d8d62d"
export CHAINSTACK_USERNAME="loving-jepsen"
export CHAINSTACK_PASSWORD="haven-shiny-purple-perch-tacky-ramp"
```

## Windows (Command Prompt)

```cmd
set CHAINSTACK_X_TOKEN=322e4e2a19ffade6ebb982ddf1d8d62d
set CHAINSTACK_USERNAME=loving-jepsen
set CHAINSTACK_PASSWORD=haven-shiny-purple-perch-tacky-ramp
```

## Windows (PowerShell)

```powershell
$env:CHAINSTACK_X_TOKEN="322e4e2a19ffade6ebb982ddf1d8d62d"
$env:CHAINSTACK_USERNAME="loving-jepsen"
$env:CHAINSTACK_PASSWORD="haven-shiny-purple-perch-tacky-ramp"
```

## Verification

After setting the environment variables, run the binary:

```bash
cargo run --release
```

You should see output like:

```
✅ VALIDATION PASSED: Transaction data received from Yellowstone
   First transaction arrived after 0.01s

✓ Transaction in slot: 408635046
  Signature: 3Y6S9oy3WnFDwqRnqwEsA4dJ4xZNXZvTyWPjFr6GXdbgMf88DUDrN3eqSSo7S1ggsQmt1M5KSLzWDN5ryK4UbqqD
```

If you see authentication errors instead, verify:
1. All three environment variables are set
2. The credentials are correct
3. You have network access to Chainstack's Yellowstone endpoint

## Architecture Rule

**CRITICAL:** This system relies exclusively on Chainstack's Yellowstone gRPC transaction streams. Do NOT implement RPC polling, websocket fallbacks, or switch to other data sources.

If authentication fails, debug the Chainstack credentials - do NOT replace the data source.
