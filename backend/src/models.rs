use serde::{Deserialize, Serialize};

// Report Logging Models
#[derive(Debug, Serialize, Deserialize)]
pub struct ReportLogRequest {
    pub report_content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportLogResponse {
    pub success: bool,
    pub message: String,
    pub transaction_signature: Option<String>,
    pub hash: Option<String>,
}

// Fuzzing Models
#[derive(Debug, Serialize, Deserialize)]
pub struct FuzzingRequest {
    pub repo_url: String,
    pub instruction_name: Option<String>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FuzzingResponse {
    pub success: bool,
    pub message: String,
    pub errors: Option<Vec<String>>,
    pub test_file: Option<String>,
    pub execution_time_ms: Option<u64>,
}

// Code Analysis Models
#[derive(Debug, Serialize, Deserialize)]
pub enum BugSeverity {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeBug {
    pub bug: String,
    pub line: u32,
    pub severity: BugSeverity,
    pub fix: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeAnalysisRequest {
    pub repo_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeAnalysisResponse {
    pub success: bool,
    pub message: String,
    pub bugs: Option<Vec<CodeBug>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubRepo {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub stargazers_count: u32,
    pub forks_count: u32,
    pub open_issues_count: u32,
    pub owner: GitHubOwner,
    pub language: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubOwner {
    pub login: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubContent {
    pub name: String,
    pub path: String,
    pub sha: String,
    pub size: Option<u64>,
    #[serde(rename = "type")]
    pub content_type: String,  // "file", "dir", "symlink", etc.
    pub download_url: Option<String>,
    pub html_url: String,
    pub content: Option<String>,
    pub encoding: Option<String>,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoIngestionRequest {
    pub repo_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoIngestionResponse {
    pub success: bool,
    pub message: String,
    pub repo: Option<GitHubRepo>,
    pub is_anchor_project: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoContentsRequest {
    pub repo_url: String,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoContentsResponse {
    pub success: bool,
    pub message: String,
    pub contents: Option<Vec<GitHubContent>>,
    pub file_content: Option<GitHubContent>,
    pub repo_url: String,
    pub path: String,
}