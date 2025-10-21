# On-Chain Report Logging Documentation

## Overview
The UseSafex platform now includes on-chain report logging functionality, which stores SHA256 hashes of scan reports on the Solana devnet blockchain. This feature enhances transparency and provides an immutable record of security scan results.

## How It Works
1. When a security scan is completed, the backend generates a SHA256 hash of the report content
2. The hash is then stored on the Solana devnet blockchain using our custom Anchor program (`report-logger`)
3. The transaction signature and hash are returned to the client for reference

## Querying Reports via Solana Explorer
You can verify the existence of a report on the blockchain by following these steps:

1. Take the transaction signature returned from the `/api/log-report` endpoint
2. Visit the Solana Explorer for devnet: https://explorer.solana.com/?cluster=devnet
3. Paste the transaction signature into the search bar
4. View the transaction details, which will include:
   - The program that was called (`report-logger`)
   - The accounts involved in the transaction
   - The report hash that was stored on-chain

## API Usage
To log a report on-chain, send a POST request to the `/api/log-report` endpoint:

```bash
curl -X POST http://localhost:8080/api/log-report \
  -H "Content-Type: application/json" \
  -d '{"report_content":"Your report content here"}'
```

The response will include:
```json
{
  "success": true,
  "message": "Report logged successfully",
  "transaction_signature": "2id1qvFo4...7iKXmqKe",
  "hash": "a591a6d40...5a3d6dbcf"
}
```

Use the `transaction_signature` to look up the transaction on Solana Explorer.