mod models;
mod github;
mod analyzer;
mod fuzzer;
mod report_logger;

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use actix_web::middleware::Logger;
use actix_web::http::header;
use actix_cors::Cors;
use models::{RepoIngestionRequest, RepoIngestionResponse, RepoContentsRequest, RepoContentsResponse, CodeAnalysisRequest, CodeAnalysisResponse, FuzzingRequest, FuzzingResponse, ReportLogRequest, ReportLogResponse};
use github::GitHubClient;
use analyzer::CodeAnalyzer;
use fuzzer::Fuzzer;
use report_logger::ReportLogger;
use tempfile::TempDir;
use git2::Repository;
use std::time::Instant;
use std::path::Path;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[post("/api/ingest-repo")]
async fn ingest_repo(repo_request: web::Json<RepoIngestionRequest>) -> impl Responder {
    let github_client = GitHubClient::new();
    
    match github_client.get_repo_from_url(&repo_request.repo_url).await {
        Ok(repo) => {
            // Check if it's an Anchor project
            let is_anchor_project = match github_client.clone_and_validate_anchor_project(&repo_request.repo_url) {
                Ok(is_anchor) => {
                    if !is_anchor {
                        // If not an Anchor project, return error
                        let response = RepoIngestionResponse {
                            success: false,
                            message: "Repository is not an Anchor project. Please provide a valid Solana Anchor project.".to_string(),
                            repo: Some(repo),
                            is_anchor_project: Some(false),
                        };
                        return HttpResponse::BadRequest().json(response);
                    }
                    Some(true)
                },
                Err(e) => {
                    let response = RepoIngestionResponse {
                        success: false,
                        message: format!("Failed to validate Anchor project: {}", e),
                        repo: Some(repo),
                        is_anchor_project: None,
                    };
                    return HttpResponse::BadRequest().json(response);
                }
            };
            
            let response = RepoIngestionResponse {
                success: true,
                message: "Anchor project successfully ingested".to_string(),
                repo: Some(repo),
                is_anchor_project,
            };
            HttpResponse::Ok().json(response)
        },
        Err(e) => {
            let response = RepoIngestionResponse {
                success: false,
                message: format!("Failed to ingest repository: {}", e),
                repo: None,
                is_anchor_project: None,
            };
            HttpResponse::BadRequest().json(response)
        }
    }
}

#[post("/api/repo-contents")]
async fn repo_contents(contents_request: web::Json<RepoContentsRequest>) -> impl Responder {
    let github_client = GitHubClient::new();
    let path_str = contents_request.path.as_deref();
    
    match github_client.get_repo_contents(&contents_request.repo_url, path_str).await {
        Ok(contents) => {
            let response = RepoContentsResponse {
                success: true,
                message: "Repository contents fetched successfully".to_string(),
                contents: Some(contents),
                file_content: None,
                repo_url: contents_request.repo_url.clone(),
                path: path_str.unwrap_or("").to_string(),
            };
            HttpResponse::Ok().json(response)
        },
        Err(e) => {
            let response = RepoContentsResponse {
                success: false,
                message: format!("Failed to fetch repository contents: {}", e),
                contents: None,
                file_content: None,
                repo_url: contents_request.repo_url.clone(),
                path: path_str.unwrap_or("").to_string(),
            };
            HttpResponse::BadRequest().json(response)
        }
    }
}

#[post("/api/fuzz-test")]
async fn fuzz_test(fuzzing_request: web::Json<FuzzingRequest>) -> impl Responder {
    let start_time = Instant::now();
    let github_client = GitHubClient::new();
    
    // Create temp directory for cloning and testing
    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(e) => {
            return HttpResponse::InternalServerError().json(FuzzingResponse {
                success: false,
                message: format!("Failed to create temporary directory: {}", e),
                errors: None,
                test_file: None,
                execution_time_ms: None,
            });
        }
    };
    
    // Clone the repository
    let repo_path = temp_dir.path().join("repo");
    match github_client.clone_repo(&fuzzing_request.repo_url, &repo_path) {
        Ok(_) => {},
        Err(e) => {
            return HttpResponse::BadRequest().json(FuzzingResponse {
                success: false,
                message: format!("Failed to clone repository: {}", e),
                errors: None,
                test_file: None,
                execution_time_ms: None,
            });
        }
    };
    
    // Initialize fuzzer
    let fuzzer = Fuzzer::new(temp_dir.path().to_path_buf());
    
    // Get instruction name or use default
    let instruction_name = fuzzing_request.instruction_name.clone().unwrap_or_else(|| "increment".to_string());
    
    // Set timeout (default to 120 seconds if not specified)
    let timeout = fuzzing_request.timeout_seconds.unwrap_or(120);
    if timeout > 120 {
        return HttpResponse::BadRequest().json(FuzzingResponse {
            success: false,
            message: "Timeout cannot exceed 120 seconds".to_string(),
            errors: None,
            test_file: None,
            execution_time_ms: None,
        });
    }
    
    // Generate and run fuzz tests
    match fuzzer.generate_and_run_fuzz_tests(&repo_path, &instruction_name) {
        Ok(result) => {
            let execution_time = start_time.elapsed().as_millis() as u64;
            
            // Get the test file content
            let test_file_path = temp_dir.path().join("fuzz_tests").join(format!("{}_fuzz_test.rs", instruction_name));
            let test_file_content = match std::fs::read_to_string(&test_file_path) {
                Ok(content) => Some(content),
                Err(_) => None,
            };
            
            HttpResponse::Ok().json(FuzzingResponse {
                success: !result.timed_out && result.errors.is_empty(),
                message: if result.timed_out {
                    "Fuzzing tests timed out".to_string()
                } else if result.errors.is_empty() {
                    "Fuzzing tests completed successfully".to_string()
                } else {
                    "Fuzzing tests found potential issues".to_string()
                },
                errors: if result.errors.is_empty() { None } else { Some(result.errors) },
                test_file: test_file_content,
                execution_time_ms: Some(execution_time),
            })
        },
        Err(e) => {
            HttpResponse::InternalServerError().json(FuzzingResponse {
                success: false,
                message: format!("Failed to run fuzzing tests: {}", e),
                errors: None,
                test_file: None,
                execution_time_ms: Some(start_time.elapsed().as_millis() as u64),
            })
        }
    }
}

#[post("/api/analyze-code")]
async fn analyze_code(analysis_request: web::Json<CodeAnalysisRequest>) -> impl Responder {
    println!("Received code analysis request for: {}", analysis_request.repo_url);
    
    // Create a temporary directory for cloning
    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(e) => {
            return HttpResponse::InternalServerError().json(CodeAnalysisResponse {
                success: false,
                message: format!("Failed to create temporary directory: {}", e),
                bugs: None,
            });
        }
    };
    
    // Clone the repository
    println!("Cloning repository to: {}", temp_dir.path().display());
    let _repo = match Repository::clone(&analysis_request.repo_url, temp_dir.path()) {
        Ok(repo) => repo,
        Err(e) => {
            return HttpResponse::BadRequest().json(CodeAnalysisResponse {
                success: false,
                message: format!("Failed to clone repository: {}", e),
                bugs: None,
            });
        }
    };
    
    // Run code analysis
    let analyzer = CodeAnalyzer::new();
    match analyzer.analyze_repo(temp_dir.path()) {
        Ok(bugs) => {
            HttpResponse::Ok().json(CodeAnalysisResponse {
                success: true,
                message: format!("Analysis completed. Found {} issues.", bugs.len()),
                bugs: Some(bugs),
            })
        },
        Err(e) => {
            HttpResponse::InternalServerError().json(CodeAnalysisResponse {
                success: false,
                message: format!("Analysis failed: {}", e),
                bugs: None,
            })
        }
    }
}

#[post("/api/log-report")]
async fn log_report(report_request: web::Json<ReportLogRequest>) -> impl Responder {
    println!("Received report logging request");
    
    // Create SHA256 hash of the report content
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(report_request.report_content.as_bytes());
    let hash = hasher.finalize();
    let hash_hex = format!("{:x}", hash);
    
    // Initialize the report logger
    match ReportLogger::new() {
        Ok(logger) => {
            // Log the report to the blockchain
            match logger.log_report(&report_request.report_content) {
                Ok(signature) => {
                    HttpResponse::Ok().json(ReportLogResponse {
                        success: true,
                        message: "Report successfully logged to Solana blockchain".to_string(),
                        transaction_signature: Some(signature),
                        hash: Some(hash_hex),
                    })
                },
                Err(e) => {
                    HttpResponse::InternalServerError().json(ReportLogResponse {
                        success: false,
                        message: format!("Failed to log report: {}", e),
                        transaction_signature: None,
                        hash: Some(hash_hex),
                    })
                }
            }
        },
        Err(e) => {
            HttpResponse::InternalServerError().json(ReportLogResponse {
                success: false,
                message: format!("Failed to initialize report logger: {}", e),
                transaction_signature: None,
                hash: None,
            })
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port: u16 = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse().unwrap_or(8080);
    println!("Starting Safex backend server at http://0.0.0.0:{port}");
    actix_web::HttpServer::new(|| {
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")
            .allowed_origin("http://localhost:3001")
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT, header::CONTENT_TYPE])
            .max_age(3600);
            
        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .service(hello)
            .service(ingest_repo)
            .service(repo_contents)
            .service(analyze_code)
            .service(fuzz_test)
            .service(log_report)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
