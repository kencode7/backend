use anyhow::Result;
use sha2::{Sha256, Digest};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;

// Program ID of the report-logger Anchor program
const PROGRAM_ID: &str = "4L6BwTs3J5deHpTLSHGPZKQKn9uhLFMKnKjhjqeobQ26";

pub struct ReportLogger {
    client: RpcClient,
    payer: Keypair,
}

impl ReportLogger {
    pub fn new() -> Result<Self> {
        // Connect to Solana devnet
        let client = RpcClient::new("https://api.devnet.solana.com".to_string());
        
        // For development, generate a new keypair
        // In production, this should be loaded from a secure location
        let payer = Keypair::new();
        
        Ok(Self { client, payer })
    }
    
    pub fn log_report(&self, report_content: &str) -> Result<String> {
        // Generate SHA256 hash of the report content
        let mut hasher = Sha256::new();
        hasher.update(report_content.as_bytes());
        let hash = hasher.finalize();
        
        // Create a new account for storing the report
        let report_account = Keypair::new();
        
        // Get program ID
        let program_id = Pubkey::from_str(PROGRAM_ID)?;
        
        // Create instruction data: [0, hash[0], hash[1], ..., hash[31]]
        // 0 is the instruction discriminator for log_report
        let mut instruction_data = vec![0];
        instruction_data.extend_from_slice(&hash);
        
        // Create the instruction
        let instruction = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(report_account.pubkey(), true),
                AccountMeta::new(self.payer.pubkey(), true),
                AccountMeta::new_readonly(Pubkey::from_str("11111111111111111111111111111111").unwrap(), false),
            ],
            data: instruction_data,
        };
        
        // Create and sign transaction
        let message = Message::new(&[instruction], Some(&self.payer.pubkey()));
        let mut transaction = Transaction::new_unsigned(message);
        
        let recent_blockhash = self.client.get_latest_blockhash()?;
        transaction.sign(&[&self.payer, &report_account], recent_blockhash);
        
        // Send transaction
        let signature = self.client.send_and_confirm_transaction(&transaction)?;
        
        // Return the transaction signature
        Ok(signature.to_string())
    }
}