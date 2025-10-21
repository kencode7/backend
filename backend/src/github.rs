use anyhow::{anyhow, Result};
use reqwest::{Client, StatusCode};
use std::time::Duration;
use std::env;
use std::fs;
use std::path::Path;
use base64;
use git2::{Repository, FetchOptions};
use tempfile::TempDir;
use toml::Table;

use crate::models::{GitHubRepo, GitHubContent};

pub struct GitHubClient {
    client: Client,
    token: Option<String>,
}

impl GitHubClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        // Try to load GitHub token from environment
        let token = env::var("GITHUB_TOKEN").ok();
        if token.is_some() {
            println!("Using GitHub token for authentication");
        } else {
            println!("No GitHub token found, using unauthenticated requests (rate limited)");
        }
        
        Self { client, token }
    }
    
    // Clone a repository to a specific path
    pub fn clone_repo(&self, repo_url: &str, target_path: &Path) -> Result<()> {
        println!("Cloning repository: {} to {}", repo_url, target_path.display());
        
        // Set up fetch options (use token if available)
        let mut fetch_opts = FetchOptions::new();
        if let Some(_token) = &self.token {
            // For authenticated cloning if needed
            fetch_opts.remote_callbacks(git2::RemoteCallbacks::new());
        }
        
        // Clone the repository
        let _repo = match Repository::clone(repo_url, target_path) {
            Ok(repo) => repo,
            Err(e) => {
                return Err(anyhow!("Failed to clone repository: {}", e));
            }
        };
        
        Ok(())
    }
    
    // Clone a repository and check if it's an Anchor project
    pub fn clone_and_validate_anchor_project(&self, repo_url: &str) -> Result<bool> {
        println!("Cloning repository: {}", repo_url);
        
        // Create a temporary directory for the clone
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path();
        
        // Set up fetch options (use token if available)
        let mut fetch_opts = FetchOptions::new();
        if let Some(_token) = &self.token {
            // For authenticated cloning if needed
            fetch_opts.remote_callbacks(git2::RemoteCallbacks::new());
        }
        
        // Clone the repository
        let _repo = match Repository::clone(repo_url, temp_path) {
            Ok(repo) => repo,
            Err(e) => {
                return Err(anyhow!("Failed to clone repository: {}", e));
            }
        };
        
        // Check if it's an Anchor project by looking for Cargo.toml with anchor-lang dependency
        self.is_anchor_project(temp_path)
    }
    
    // Check if a repository is an Anchor project
    fn is_anchor_project(&self, repo_path: &Path) -> Result<bool> {
        // Look for Cargo.toml files
        let cargo_paths = self.find_cargo_toml_files(repo_path)?;
        
        // Check each Cargo.toml for anchor-lang dependency
        for cargo_path in cargo_paths {
            if self.has_anchor_dependency(&cargo_path)? {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    // Find all Cargo.toml files in the repository recursively
    fn find_cargo_toml_files(&self, repo_path: &Path) -> Result<Vec<String>> {
        let mut cargo_files = Vec::new();
        self.find_cargo_toml_recursive(repo_path, &mut cargo_files)?;
        println!("Found {} Cargo.toml files", cargo_files.len());
        Ok(cargo_files)
    }
    
    // Recursively search for Cargo.toml files
    fn find_cargo_toml_recursive(&self, dir_path: &Path, cargo_files: &mut Vec<String>) -> Result<()> {
        if !dir_path.is_dir() {
            return Ok(());
        }
        
        // Check for Cargo.toml in current directory
        let cargo_path = dir_path.join("Cargo.toml");
        if cargo_path.exists() {
            cargo_files.push(cargo_path.to_string_lossy().to_string());
            println!("Found Cargo.toml at: {}", cargo_path.display());
        }
        
        // Recursively check subdirectories
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            // Skip hidden directories and files
            if path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with('.'))
                .unwrap_or(false) {
                continue;
            }
            
            if path.is_dir() {
                self.find_cargo_toml_recursive(&path, cargo_files)?;
            }
        }
        
        Ok(())
    }
    
    // Check if a Cargo.toml file has anchor-lang dependency
    fn has_anchor_dependency(&self, cargo_path: &str) -> Result<bool> {
        let content = fs::read_to_string(cargo_path)?;
        
        // Parse TOML
        let cargo_toml: Table = match content.parse() {
            Ok(toml) => toml,
            Err(e) => {
                println!("Failed to parse Cargo.toml: {}", e);
                return Ok(false);
            }
        };
        
        // Check for anchor-lang in dependencies
        if let Some(deps) = cargo_toml.get("dependencies") {
            if let Some(deps_table) = deps.as_table() {
                if deps_table.contains_key("anchor-lang") {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }

    pub async fn get_repo_from_url(&self, repo_url: &str) -> Result<GitHubRepo> {
        // Extract owner and repo name from URL
        let (owner, repo) = self.extract_owner_repo(repo_url)?;
        println!("Fetching repo: owner={}, repo={}", owner, repo);
        self.get_repo(owner, repo).await
    }

    pub async fn get_repo(&self, owner: &str, repo: &str) -> Result<GitHubRepo> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        println!("Making API request to: {}", url);
        
        let mut request = self.client
            .get(&url)
            .header("User-Agent", "Safex-App")
            .header("Accept", "application/vnd.github.v3+json");
        
        // Add authorization if token is available
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("token {}", token));
        }
        
        let response = match request.send().await {
            Ok(resp) => resp,
            Err(e) => {
                println!("Network error: {}", e);
                return Err(anyhow!("Failed to connect to GitHub API: {}", e));
            }
        };

        let status = response.status();
        println!("GitHub API response status: {}", status);
        
        if status == StatusCode::NOT_FOUND {
            return Err(anyhow!("Repository not found: {}/{}", owner, repo));
        } else if status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS {
            return Err(anyhow!("GitHub API rate limit exceeded. Please try again later or add a GitHub token."));
        } else if !status.is_success() {
            let error_text = match response.text().await {
                Ok(text) => text,
                Err(_) => "Could not read error response".to_string()
            };
            println!("GitHub API error: {} - {}", status, error_text);
            return Err(anyhow!("GitHub API error: {} - {}", status, error_text));
        }

        match response.json::<GitHubRepo>().await {
            Ok(repo_data) => Ok(repo_data),
            Err(e) => {
                println!("Failed to parse GitHub response: {}", e);
                Err(anyhow!("Failed to parse GitHub repository data: {}", e))
            }
        }
    }

    pub async fn get_repo_contents(&self, repo_url: &str, path: Option<&str>) -> Result<Vec<GitHubContent>> {
        let (owner, repo) = self.extract_owner_repo(repo_url)?;
        let path = path.unwrap_or("");
        
        let url = format!("https://api.github.com/repos/{}/{}/contents/{}", owner, repo, path);
        println!("Fetching repo contents: {}", url);
        
        let mut request = self.client
            .get(&url)
            .header("User-Agent", "Safex-App")
            .header("Accept", "application/vnd.github.v3+json");
        
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("token {}", token));
        }
        
        let response = match request.send().await {
            Ok(resp) => resp,
            Err(e) => return Err(anyhow!("Failed to connect to GitHub API: {}", e)),
        };
        
        let status = response.status();
        if !status.is_success() {
            let error_text = match response.text().await {
                Ok(text) => text,
                Err(_) => "Could not read error response".to_string()
            };
            return Err(anyhow!("GitHub API error: {} - {}", status, error_text));
        }
        
        // GitHub API returns either an array (for directories) or a single object (for files)
        if response.headers().get("content-type").map_or(false, |ct| ct.to_str().unwrap_or("").contains("application/json")) {
            let text = response.text().await?;
            
            // Try to parse as array first
            match serde_json::from_str::<Vec<GitHubContent>>(&text) {
                Ok(contents) => {
                    return Ok(contents);
                },
                Err(_) => {
                    // If not an array, try to parse as a single file
                    match serde_json::from_str::<GitHubContent>(&text) {
                        Ok(file) => {
                            // If it's a file, decode the content if present
                            let mut file = file;
                            if let (Some(content), Some(encoding)) = (&file.content, &file.encoding) {
                                if encoding == "base64" {
                                    // Remove whitespace and newlines from base64 content
                                    let clean_content = content.replace("\n", "");
                                    match base64::decode(&clean_content) {
                                        Ok(decoded) => {
                                            match String::from_utf8(decoded) {
                                                Ok(text) => file.content = Some(text),
                                                Err(_) => println!("Content is not valid UTF-8")
                                            }
                                        },
                                        Err(e) => println!("Failed to decode base64: {}", e)
                                    }
                                }
                            }
                            return Ok(vec![file]);
                        },
                        Err(e) => return Err(anyhow!("Failed to parse GitHub content: {}", e)),
                    }
                }
            }
        }
        
        Err(anyhow!("Unexpected response format from GitHub API"))
    }
    
    fn extract_owner_repo<'a>(&self, repo_url: &'a str) -> Result<(&'a str, &'a str)> {
        // Extract owner and repo name from URL
        // Example: https://github.com/owner/repo
        let url = repo_url.trim_end_matches('/');
        let parts: Vec<&str> = url.split('/').collect();
        
        println!("URL parts: {:?}", parts);
        
        // Handle different URL formats
        // Format 1: https://github.com/owner/repo
        // Format 2: http://github.com/owner/repo
        // Format 3: github.com/owner/repo
        
        if parts.len() >= 5 && (parts[2] == "github.com" || parts[2].contains("github")) {
            // Full URL with https://
            Ok((parts[3], parts[4]))
        } else if parts.len() >= 3 && (parts[0] == "github.com" || parts[0].contains("github")) {
            // URL without protocol
            Ok((parts[1], parts[2]))
        } else {
            Err(anyhow!("Invalid GitHub repository URL: {}", repo_url))
        }
    }
}