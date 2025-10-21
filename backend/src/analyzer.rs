use anyhow::{anyhow, Result};
use regex::Regex;
use std::path::Path;
use std::process::Command;

use crate::models::{CodeBug, BugSeverity};

pub struct CodeAnalyzer;

impl CodeAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    // Run analysis on the repository
    pub fn analyze_repo(&self, repo_path: &Path) -> Result<Vec<CodeBug>> {
        println!("Analyzing repository at: {}", repo_path.display());
        
        // Create a default set of bugs in case analysis fails
        let mut all_bugs = Vec::new();
        
        // Try to run cargo clippy
        match self.run_cargo_clippy(repo_path) {
            Ok(clippy_bugs) => all_bugs.extend(clippy_bugs),
            Err(e) => {
                println!("Warning: Cargo clippy analysis failed: {}", e);
                // Add a placeholder bug to indicate the failure
                all_bugs.push(CodeBug {
                    bug: "Failed to run Cargo clippy analysis".to_string(),
                    line: 0,
                    severity: BugSeverity::Low,
                    fix: "Ensure Cargo and Clippy are installed and the project is a valid Rust project".to_string(),
                });
            }
        }
        
        // Try to run custom Anchor lints
        match self.run_anchor_lints(repo_path) {
            Ok(anchor_bugs) => all_bugs.extend(anchor_bugs),
            Err(e) => {
                println!("Warning: Anchor lints analysis failed: {}", e);
                // Add a placeholder bug to indicate the failure
                all_bugs.push(CodeBug {
                    bug: "Failed to run Anchor-specific lints".to_string(),
                    line: 0,
                    severity: BugSeverity::Low,
                    fix: "Ensure the project is a valid Anchor project".to_string(),
                });
            }
        }
        
        // Always return success with whatever bugs we found
        Ok(all_bugs)
    }
    
    // Run cargo clippy and parse its output
    fn run_cargo_clippy(&self, repo_path: &Path) -> Result<Vec<CodeBug>> {
        println!("Running cargo clippy...");
        
        let output = Command::new("cargo")
            .args(["clippy", "--message-format=json"])
            .current_dir(repo_path)
            .output()?;
            
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse clippy JSON output
        self.parse_clippy_output(&stdout)
    }
    
    // Parse clippy JSON output to extract warnings
    fn parse_clippy_output(&self, clippy_output: &str) -> Result<Vec<CodeBug>> {
        let mut bugs = Vec::new();
        
        for line in clippy_output.lines() {
            if line.trim().is_empty() {
                continue;
            }
            
            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(json) => {
                    if let Some(message) = json.get("message") {
                        if let (Some(text), Some(level)) = (message.get("message"), message.get("level")) {
                            if level.as_str() == Some("warning") || level.as_str() == Some("error") {
                                let bug_text = text.as_str().unwrap_or("Unknown issue").to_string();
                                
                                // Extract line number
                                let line_num = if let Some(spans) = message.get("spans") {
                                    if let Some(span) = spans.as_array().and_then(|s| s.first()) {
                                        span.get("line_start").and_then(|l| l.as_u64()).unwrap_or(0) as u32
                                    } else {
                                        0
                                    }
                                } else {
                                    0
                                };
                                
                                // Determine severity
                                let severity = if bug_text.contains("unsafe") {
                                    BugSeverity::High
                                } else if bug_text.contains("unused") {
                                    BugSeverity::Low
                                } else {
                                    BugSeverity::Medium
                                };
                                
                                // Generate fix suggestion
                                let fix = self.suggest_fix(&bug_text);
                                
                                bugs.push(CodeBug {
                                    bug: bug_text,
                                    line: line_num,
                                    severity,
                                    fix,
                                });
                            }
                        }
                    }
                },
                Err(e) => {
                    println!("Warning: Failed to parse clippy JSON output: {}", e);
                    // Continue processing other lines
                }
            }
        }
        
        // If we didn't find any bugs but there was output, add a default bug
        if bugs.is_empty() && !clippy_output.trim().is_empty() {
            bugs.push(CodeBug {
                bug: "Clippy output could not be parsed".to_string(),
                line: 0,
                severity: BugSeverity::Low,
                fix: "Check the project structure and ensure it's a valid Rust project".to_string(),
            });
        }
        
        Ok(bugs)
    }
    
    // Run custom Anchor-specific lints
    fn run_anchor_lints(&self, repo_path: &Path) -> Result<Vec<CodeBug>> {
        println!("Running custom Anchor lints...");
        
        let mut bugs = Vec::new();
        
        // Check for missing #[account(signer)]
        match self.check_missing_signer_attribute(repo_path, &mut bugs) {
            Ok(_) => {},
            Err(e) => {
                println!("Warning: Failed to check for missing signer attributes: {}", e);
                // Add a placeholder bug
                bugs.push(CodeBug {
                    bug: "Failed to check for missing #[account(signer)] attributes".to_string(),
                    line: 0,
                    severity: BugSeverity::Medium,
                    fix: "Manually review your code for missing signer attributes".to_string(),
                });
            }
        }
        
        Ok(bugs)
    }
    
    // Check for missing #[account(signer)] attribute
    fn check_missing_signer_attribute(&self, repo_path: &Path, bugs: &mut Vec<CodeBug>) -> Result<()> {
        // Find all Rust files in the project
        let rust_files = self.find_rust_files(repo_path)?;
        
        for file_path in rust_files {
            let content = match std::fs::read_to_string(&file_path) {
                Ok(content) => content,
                Err(e) => {
                    println!("Warning: Failed to read file {}: {}", file_path, e);
                    continue;
                }
            };
            
            // Look for patterns that might indicate missing signer attribute
            let re_account_struct = Regex::new(r"pub\s+struct\s+(\w+)\s*\{").unwrap();
            let re_signer_check = Regex::new(r"#\[account\(.*signer.*\)\]").unwrap();
            
            // Find account structs
            for cap in re_account_struct.captures_iter(&content) {
                let struct_name = &cap[1];
                
                // Check if the struct is used as a signer in any instruction
                if content.contains(&format!("{}: &Signer", struct_name)) || 
                   content.contains(&format!("{}: Signer", struct_name)) {
                    
                    // Check if it has the signer attribute
                    if !re_signer_check.is_match(&content) {
                        // Get approximate line number
                        let line_num = content[..cap.get(0).unwrap().start()]
                            .lines()
                            .count() as u32 + 1;
                            
                        bugs.push(CodeBug {
                            bug: format!("Missing #[account(signer)] attribute for {}", struct_name),
                            line: line_num,
                            severity: BugSeverity::High,
                            fix: format!("Add #[account(signer)] attribute to the {} struct", struct_name),
                        });
                    }
                }
            }
        }
        
        Ok(())
    }
    
    // Find all Rust files in the project
    fn find_rust_files(&self, dir_path: &Path) -> Result<Vec<String>> {
        let mut rust_files = Vec::new();
        
        if !dir_path.is_dir() {
            return Ok(rust_files);
        }
        
        for entry in std::fs::read_dir(dir_path)? {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    println!("Warning: Failed to read directory entry: {}", e);
                    continue;
                }
            };
            let path = entry.path();
            
            // Skip hidden directories and files
            if path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with('.'))
                .unwrap_or(false) {
                continue;
            }
            
            if path.is_dir() {
                match self.find_rust_files(&path) {
                    Ok(mut subdir_files) => rust_files.append(&mut subdir_files),
                    Err(e) => {
                        println!("Warning: Failed to search directory {}: {}", path.display(), e);
                        continue;
                    }
                }
            } else if let Some(extension) = path.extension() {
                if extension == "rs" {
                    rust_files.push(path.to_string_lossy().to_string());
                }
            }
        }
        
        Ok(rust_files)
    }
    
    // Suggest fixes based on the bug description
    fn suggest_fix(&self, bug_description: &str) -> String {
        if bug_description.contains("unused variable") {
            "Remove the unused variable or prefix it with an underscore (_)".to_string()
        } else if bug_description.contains("unused import") {
            "Remove the unused import".to_string()
        } else if bug_description.contains("unsafe") {
            "Avoid using unsafe code, use safe alternatives".to_string()
        } else {
            "Review the code and fix the issue according to best practices".to_string()
        }
    }
}