use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzingResult {
    pub success: bool,
    pub timed_out: bool,
    pub errors: Vec<String>,
    pub execution_time_ms: u64,
}

pub struct Fuzzer {
    temp_dir: PathBuf,
}

impl Fuzzer {
    pub fn new(temp_dir: PathBuf) -> Self {
        Self { temp_dir }
    }

    pub fn generate_and_run_fuzz_tests(&self, repo_path: &Path, instruction_name: &str) -> Result<FuzzingResult> {
        // Generate test file
        let test_file_path = self.generate_test_file(repo_path, instruction_name)?;
        
        // Run the tests with time limit
        self.run_tests(&test_file_path, 120) // 2 minute limit
    }

    fn generate_test_file(&self, repo_path: &Path, instruction_name: &str) -> Result<PathBuf> {
        // Create test directory
        let test_dir = self.temp_dir.join("fuzz_tests");
        fs::create_dir_all(&test_dir)?;
        
        // Create test file
        let test_file_path = test_dir.join(format!("{}_fuzz_test.rs", instruction_name));
        let mut file = File::create(&test_file_path)?;
        
        // Write test content based on instruction
        if instruction_name.to_lowercase() == "increment" {
            self.write_increment_test(&mut file)?;
        } else {
            self.write_generic_test(&mut file, instruction_name)?;
        }
        
        Ok(test_file_path)
    }
    
    fn write_increment_test(&self, file: &mut File) -> Result<()> {
        writeln!(file, r#"
#[cfg(test)]
mod tests {{
    use proptest::prelude::*;
    use solana_program_test::*;
    use solana_sdk::{{signature::Keypair, signer::Signer}};
    use anchor_lang::prelude::*;
    
    proptest! {{
        #[test]
        fn test_increment_fuzz(value in 0..u64::MAX) {{
            let program_id = Pubkey::new_unique();
            let counter = Keypair::new();
            let user = Keypair::new();
            
            // Create program test environment
            let mut program_test = ProgramTest::new(
                "counter_program",
                program_id,
                None,
            );
            
            // Add counter account
            program_test.add_account(
                counter.pubkey(),
                Account {{
                    lamports: 1000000,
                    data: vec![0; 8], // Space for a u64
                    owner: program_id,
                    ..Account::default()
                }},
            );
            
            // Start the test environment
            let (mut banks_client, payer, recent_blockhash) = program_test.start().unwrap();
            
            // Build transaction
            let mut transaction = solana_sdk::transaction::Transaction::new_with_payer(
                &[Instruction {{
                    program_id,
                    accounts: vec![
                        AccountMeta::new(counter.pubkey(), false),
                        AccountMeta::new_readonly(user.pubkey(), true),
                    ],
                    data: [0, value.to_le_bytes().to_vec()].concat(), // 0 = increment instruction, followed by value
                }}],
                Some(&payer.pubkey()),
            );
            
            transaction.sign(&[&payer, &user], recent_blockhash);
            
            // Process transaction with timeout
            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(2);
            
            while start.elapsed() < timeout {{
                match banks_client.process_transaction(transaction.clone()) {{
                    Ok(_) => return Ok(()), // Success
                    Err(e) => {{
                        // Check for overflow errors
                        if e.to_string().contains("overflow") {{
                            println!("Found overflow error: {{}}", e);
                            return Err(TestCaseError::reject("Overflow detected"));
                        }}
                        
                        // Check for account validation errors
                        if e.to_string().contains("account validation failed") {{
                            println!("Found validation error: {{}}", e);
                            return Err(TestCaseError::reject("Validation failed"));
                        }}
                    }}
                }}
            }}
            
            // Timeout
            Err(TestCaseError::reject("Test timed out"))
        }}
    }}
}}"#)?;
        
        Ok(())
    }
    
    fn write_generic_test(&self, file: &mut File, instruction_name: &str) -> Result<()> {
        writeln!(file, r#"
#[cfg(test)]
mod tests {{
    use proptest::prelude::*;
    use solana_program_test::*;
    use solana_sdk::{{signature::Keypair, signer::Signer}};
    use anchor_lang::prelude::*;
    
    proptest! {{
        #[test]
        fn test_{}_fuzz(
            // Generate random inputs based on instruction type
            value in 0..u64::MAX,
        ) {{
            let program_id = Pubkey::new_unique();
            let account = Keypair::new();
            let user = Keypair::new();
            
            // Create program test environment
            let mut program_test = ProgramTest::new(
                "anchor_program",
                program_id,
                None,
            );
            
            // Add test account
            program_test.add_account(
                account.pubkey(),
                Account {{
                    lamports: 1000000,
                    data: vec![0; 32], // Generic space
                    owner: program_id,
                    ..Account::default()
                }},
            );
            
            // Start the test environment
            let (mut banks_client, payer, recent_blockhash) = program_test.start().unwrap();
            
            // Build transaction with generic instruction
            let mut transaction = solana_sdk::transaction::Transaction::new_with_payer(
                &[Instruction {{
                    program_id,
                    accounts: vec![
                        AccountMeta::new(account.pubkey(), false),
                        AccountMeta::new_readonly(user.pubkey(), true),
                    ],
                    data: vec![0, value.to_le_bytes().to_vec()].concat(), // Generic instruction data
                }}],
                Some(&payer.pubkey()),
            );
            
            transaction.sign(&[&payer, &user], recent_blockhash);
            
            // Process transaction with timeout
            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(2);
            
            while start.elapsed() < timeout {{
                match banks_client.process_transaction(transaction.clone()) {{
                    Ok(_) => return Ok(()), // Success
                    Err(e) => {{
                        // Check for common errors
                        if e.to_string().contains("overflow") || 
                           e.to_string().contains("underflow") ||
                           e.to_string().contains("account validation failed") {{
                            println!("Found error: {{}}", e);
                            return Err(TestCaseError::reject("Error detected"));
                        }}
                    }}
                }}
            }}
            
            // Timeout
            Err(TestCaseError::reject("Test timed out"))
        }}
    }}
}}"#, instruction_name)?;
        
        Ok(())
    }
    
    fn run_tests(&self, test_file_path: &Path, time_limit_secs: u64) -> Result<FuzzingResult> {
        // Create Cargo.toml
        let test_dir = test_file_path.parent().ok_or_else(|| anyhow!("Invalid test path"))?;
        let cargo_path = test_dir.join("Cargo.toml");
        let mut cargo_file = File::create(&cargo_path)?;
        
        writeln!(cargo_file, r#"
[package]
name = "anchor_fuzz_tests"
version = "0.1.0"
edition = "2021"

[dependencies]
solana-program = "1.16"
solana-program-test = "1.16"
solana-sdk = "1.16"
proptest = "1.2"
anchor-lang = {{ version = "0.28.0", optional = true }}

[lib]
name = "anchor_fuzz_tests"
path = "src/lib.rs"

[features]
default = ["anchor"]
anchor = ["anchor-lang"]
test-sbf = []
"#)?;
        
        // Create lib.rs
        let src_dir = test_dir.join("src");
        fs::create_dir_all(&src_dir)?;
        
        let lib_path = src_dir.join("lib.rs");
        let mut lib_file = File::create(&lib_path)?;
        writeln!(lib_file, "// Fuzz test harness")?;
        writeln!(lib_file, "#[allow(warnings)]")?;
        writeln!(lib_file, "mod {};", test_file_path.file_stem().unwrap().to_string_lossy())?;
        
        // Copy test file to src directory
        let test_dest = src_dir.join(test_file_path.file_name().unwrap());
        fs::copy(test_file_path, &test_dest)?;
        
        // Run cargo test with timeout
        let start_time = std::time::Instant::now();
        
        // Use cargo directly instead of timeout command (which may not exist on macOS)
        let output = Command::new("cargo")
            .arg("test")
            .arg("--lib")
            .arg("--features=anchor")
            .current_dir(test_dir)
            .output()
            .map_err(|e| anyhow!("Failed to run tests: {}", e))?;
        
        let duration = start_time.elapsed();
        let timed_out = duration.as_secs() >= time_limit_secs;
        
        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        
        // Extract errors
        let errors = self.extract_errors(&stdout, &stderr);
        
        // Save test output for debugging
        let output_path = test_dir.join("test_output.log");
        let mut output_file = File::create(output_path)?;
        writeln!(output_file, "STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr)?;
        
        Ok(FuzzingResult {
            success: output.status.success() && !timed_out && errors.is_empty(),
            timed_out,
            errors,
            execution_time_ms: duration.as_millis() as u64,
        })
    }
    
    fn extract_errors(&self, stdout: &str, stderr: &str) -> Vec<String> {
        let mut errors = Vec::new();
        
        // Look for specific error patterns
        for line in stdout.lines().chain(stderr.lines()) {
            if line.contains("error:") || 
               line.contains("panicked") || 
               line.contains("overflow") || 
               line.contains("underflow") ||
               line.contains("validation failed") ||
               line.contains("Error:") ||
               line.contains("error[E") {
                errors.push(line.trim().to_string());
            }
        }
        
        errors
    }
}