"use client";

import { useState } from 'react';
import Image from 'next/image';

interface GitHubRepo {
  id: number;
  name: string;
  full_name: string;
  description?: string;
  html_url: string;
  stargazers_count: number;
  forks_count: number;
  open_issues_count: number;
  owner: {
    login: string;
    avatar_url?: string;
  };
  language?: string;
  created_at: string;
  updated_at: string;
}

interface RepoIngestionResponse {
  success: boolean;
  message: string;
  repo?: GitHubRepo;
  is_anchor_project?: boolean;
}

interface GitHubContent {
  name: string;
  path: string;
  content_type: string;
  size?: number;
  html_url: string;
  download_url?: string;
  content?: string;
  encoding?: string;
  sha: string;
  url: string;
}

interface CodeBug {
  bug: string;
  line: number;
  severity: 'low' | 'medium' | 'high';
  fix: string;
}

interface CodeAnalysisResponse {
  success: boolean;
  message: string;
  bugs?: CodeBug[];
}

interface FuzzingResponse {
  success: boolean;
  message: string;
  errors?: string[];
  test_file?: string;
  execution_time_ms?: number;
}

interface RepoContentsResponse {
  success: boolean;
  message: string;
  contents?: GitHubContent[];
  file_content?: GitHubContent;
  repo_url: string;
  path: string;
}

export default function Home() {
  const [repoUrl, setRepoUrl] = useState('');
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<any>(null);
  const [error, setError] = useState('');
  const [currentPath, setCurrentPath] = useState('');
  const [contentsResult, setContentsResult] = useState<RepoContentsResponse | null>(null);
  const [loadingContents, setLoadingContents] = useState(false);
  const [analysisResult, setAnalysisResult] = useState<CodeAnalysisResponse | null>(null);
  const [loadingAnalysis, setLoadingAnalysis] = useState(false);
  const [fuzzingResult, setFuzzingResult] = useState<FuzzingResponse | null>(null);
  const [loadingFuzzing, setLoadingFuzzing] = useState(false);
  const [instructionName, setInstructionName] = useState('increment');
  const [timeoutSeconds, setTimeoutSeconds] = useState(60);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError('');
    setResult(null);
    setContentsResult(null);
    setCurrentPath('');
    setAnalysisResult(null);

    try {
      const response = await fetch('http://127.0.0.1:8080/api/ingest-repo', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ repo_url: repoUrl }),
      });

      const data = await response.json();
      setResult(data);
      
      if (!response.ok) {
        throw new Error(data.message || 'Failed to ingest repository');
      }

      // Only fetch contents if it's a valid Anchor project
      if (data.success && data.is_anchor_project) {
        fetchRepoContents(repoUrl, '');
      }
    } catch (err: any) {
      setError(err.message || 'An error occurred');
    } finally {
      setLoading(false);
    }
  };

  const analyzeCode = async () => {
    if (!repoUrl) return;
    
    setLoadingAnalysis(true);
    setError('');
    setAnalysisResult(null);
    
    try {
      const response = await fetch('http://127.0.0.1:8080/api/analyze-code', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ repo_url: repoUrl }),
      });

      // Check if response is ok before trying to parse JSON
      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(errorText || `Server error: ${response.status}`);
      }
      
      // Check if response has content before parsing
      const responseText = await response.text();
      if (!responseText || responseText.trim() === '') {
        throw new Error('Empty response from server');
      }
      
      // Parse the JSON response
      try {
        const data = JSON.parse(responseText);
        setAnalysisResult(data);
      } catch (jsonError) {
        console.error('JSON parsing error:', jsonError);
        throw new Error('Invalid response format from server');
      }
    } catch (err: any) {
      console.error('Analysis error:', err);
      setError(err.message || 'An error occurred during code analysis');
      // Set a fallback analysis result to show the error
      setAnalysisResult({
        success: false,
        message: err.message || 'Failed to analyze code',
        bugs: []
      });
    } finally {
      setLoadingAnalysis(false);
    }
  };
  
  const runFuzzingTests = async () => {
    if (!repoUrl) return;
    
    setLoadingFuzzing(true);
    setFuzzingResult(null);
    setError('');
    
    try {
      const response = await fetch('http://127.0.0.1:8080/api/fuzz-test', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ 
          repo_url: repoUrl,
          instruction_name: instructionName,
          timeout_seconds: timeoutSeconds
        }),
      });

      if (!response.ok) {
        const text = await response.text();
        throw new Error(text || 'Failed to run fuzzing tests');
      }

      const data = await response.json();
      setFuzzingResult(data);
    } catch (err: any) {
      setError(err.message || 'Failed to run fuzzing tests');
    } finally {
      setLoadingFuzzing(false);
    }
  };

  const fetchRepoContents = async (repoUrl: string, path: string) => {
    setLoadingContents(true);
    setError('');
    
    try {
      const response = await fetch('http://127.0.0.1:8080/api/repo-contents', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ repo_url: repoUrl, path: path || undefined }),
      });

      // Check if response is ok before trying to parse JSON
      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(errorText || `Server error: ${response.status}`);
      }
      
      // Check if response has content before parsing
      const responseText = await response.text();
      if (!responseText || responseText.trim() === '') {
        throw new Error('Empty response from server');
      }
      
      // Parse the JSON response
      try {
        const data = JSON.parse(responseText);
        setContentsResult(data);
        setCurrentPath(path);
      } catch (jsonError) {
        console.error('JSON parsing error:', jsonError);
        throw new Error('Invalid response format from server');
      }
    } catch (err: any) {
      console.error('Repository contents error:', err);
      setError(err.message || 'An error occurred while fetching contents');
      // Set a fallback contents result to show the error
      setContentsResult({
        success: false,
        message: err.message || 'Failed to fetch repository contents',
        repo_url: repoUrl,
        path: path
      });
    } finally {
      setLoadingContents(false);
    }
  };

  const navigateToPath = (path: string) => {
    fetchRepoContents(repoUrl, path);
  };

  const navigateUp = () => {
    if (!currentPath) return;
    
    const pathParts = currentPath.split('/');
    pathParts.pop();
    const parentPath = pathParts.join('/');
    
    fetchRepoContents(repoUrl, parentPath);
  };

  const getFileIcon = (contentType: string) => {
    switch (contentType) {
      case 'dir':
        return '/folder.svg';
      case 'file':
        return '/file.svg';
      default:
        return '/file.svg';
    }
  };

  const formatFileSize = (bytes?: number) => {
    if (bytes === undefined) return 'Unknown size';
    
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div className="flex min-h-screen flex-col items-center justify-center p-8">
      <h1 className="text-6xl font-bold mb-8">Safex</h1>
      
      <div className="w-full max-w-4xl bg-gray-800 p-6 rounded-lg shadow-lg">
        <h2 className="text-2xl font-semibold mb-4">GitHub Repository Browser</h2>
        
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label htmlFor="repoUrl" className="block text-sm font-medium mb-1">
              GitHub Repository URL
            </label>
            <input
              type="text"
              id="repoUrl"
              value={repoUrl}
              onChange={(e) => setRepoUrl(e.target.value)}
              placeholder="https://github.com/owner/repo"
              className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
              required
            />
          </div>
          
          <button
            type="submit"
            disabled={loading}
            className="w-full bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
          >
            {loading ? 'Processing...' : 'Browse Repository'}
          </button>
        </form>
        
        {error && (
          <div className="mt-4 p-3 bg-red-900/50 border border-red-700 rounded-md text-red-200">
            {error}
          </div>
        )}
        
        {result && (
          <div className="mt-4 p-4 bg-gray-700 rounded-md">
            <h3 className="text-xl font-medium mb-2">{result.repo.name}</h3>
            <p className="text-gray-300 mb-2">{result.repo.description || 'No description'}</p>
            <div className="flex space-x-4 text-sm text-gray-400">
              <span>⭐ {result.repo.stargazers_count}</span>
              <span>🍴 {result.repo.forks_count}</span>
              {result.repo.language && <span>🔤 {result.repo.language}</span>}
            </div>
            {result.is_anchor_project !== undefined && (
              <div className={`mt-2 p-2 rounded-md ${result.is_anchor_project ? 'bg-green-900/50 border border-green-700 text-green-200' : 'bg-yellow-900/50 border border-yellow-700 text-yellow-200'}`}>
                <span className="font-medium">{result.is_anchor_project ? '✓ Valid Anchor Project' : '✗ Not an Anchor Project'}</span>
                {!result.is_anchor_project && <p className="text-sm mt-1">Please provide a repository that contains Anchor smart contracts.</p>}
              </div>
            )}
            {result.is_anchor_project && (
              <div className="mt-3 space-y-3">
                <button
                  onClick={analyzeCode}
                  disabled={loadingAnalysis}
                  className="bg-purple-600 hover:bg-purple-700 text-white font-medium py-2 px-4 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 disabled:opacity-50"
                >
                  {loadingAnalysis ? 'Analyzing Code...' : 'Analyze Code for Security Issues'}
                </button>
                
                <div className="p-3 bg-gray-800 rounded-md">
                  <h4 className="text-md font-medium mb-2">Lightweight Fuzzing</h4>
                  <div className="space-y-2">
                    <div>
                      <label htmlFor="instructionName" className="block text-sm font-medium mb-1">
                        Instruction Name
                      </label>
                      <input
                        type="text"
                        id="instructionName"
                        value={instructionName}
                        onChange={(e) => setInstructionName(e.target.value)}
                        placeholder="increment"
                        className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                      />
                    </div>
                    <div>
                      <label htmlFor="timeoutSeconds" className="block text-sm font-medium mb-1">
                        Timeout (seconds, max 120)
                      </label>
                      <input
                        type="number"
                        id="timeoutSeconds"
                        value={timeoutSeconds}
                        onChange={(e) => setTimeoutSeconds(Number(e.target.value))}
                        min="1"
                        max="120"
                        className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                      />
                    </div>
                    <button
                      onClick={runFuzzingTests}
                      disabled={loadingFuzzing}
                      className="w-full bg-green-600 hover:bg-green-700 text-white font-medium py-2 px-4 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500 disabled:opacity-50"
                    >
                      {loadingFuzzing ? 'Running Fuzz Tests...' : 'Run Fuzz Tests'}
                    </button>
                  </div>
                </div>
              </div>
            )}
            <a 
              href={result.repo.html_url} 
              target="_blank" 
              rel="noopener noreferrer"
              className="mt-3 inline-block text-blue-400 hover:text-blue-300"
            >
              View on GitHub →
            </a>
          </div>
        )}
        
        {loadingAnalysis && (
          <div className="mt-4 p-4 flex justify-center">
            <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-purple-500"></div>
            <span className="ml-2">Running security analysis...</span>
          </div>
        )}
        
        {loadingFuzzing && (
          <div className="mt-4 p-4 flex justify-center">
            <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-green-500"></div>
            <span className="ml-2">Running fuzzing tests...</span>
          </div>
        )}
        
        {fuzzingResult && (
          <div className="mt-4 bg-gray-700 rounded-md overflow-hidden">
            <div className={`p-3 border-b border-gray-600 ${
              fuzzingResult.success ? 'bg-green-800' : 'bg-red-800'
            }`}>
              <h3 className="text-xl font-medium">Fuzzing Test Results</h3>
              <p className="text-sm text-gray-300">{fuzzingResult.message}</p>
              {fuzzingResult.execution_time_ms && (
                <p className="text-xs text-gray-400">Execution time: {(fuzzingResult.execution_time_ms / 1000).toFixed(2)}s</p>
              )}
            </div>
            
            {fuzzingResult.errors && fuzzingResult.errors.length > 0 && (
              <div className="p-4">
                <h4 className="font-medium mb-2">Detected Issues:</h4>
                <ul className="list-disc pl-5 space-y-1">
                  {fuzzingResult.errors.map((error, index) => (
                    <li key={index} className="text-red-300">{error}</li>
                  ))}
                </ul>
              </div>
            )}
            
            {fuzzingResult.test_file && (
              <div className="p-4">
                <h4 className="font-medium mb-2">Generated Test File:</h4>
                <pre className="bg-gray-800 p-3 rounded-md overflow-x-auto text-sm font-mono text-gray-300">
                  {fuzzingResult.test_file}
                </pre>
              </div>
            )}
          </div>
        )}
        
        {analysisResult && (
          <div className="mt-4 bg-gray-700 rounded-md overflow-hidden">
            <div className="bg-gray-800 p-3 border-b border-gray-600">
              <h3 className="text-xl font-medium">Code Analysis Results</h3>
              <p className="text-sm text-gray-300">{analysisResult.message}</p>
            </div>
            
            {analysisResult.bugs && analysisResult.bugs.length > 0 ? (
              <div className="divide-y divide-gray-600">
                {analysisResult.bugs.map((bug, index) => (
                  <div key={index} className="p-4">
                    <div className="flex items-center justify-between mb-2">
                      <h4 className="font-medium">{bug.bug}</h4>
                      <span className={`px-2 py-1 text-xs rounded-full ${
                        bug.severity === 'high' ? 'bg-red-900 text-red-200' :
                        bug.severity === 'medium' ? 'bg-yellow-900 text-yellow-200' :
                        'bg-blue-900 text-blue-200'
                      }`}>
                        {bug.severity.toUpperCase()}
                      </span>
                    </div>
                    <p className="text-sm text-gray-300 mb-2">Line: {bug.line}</p>
                    <div className="bg-gray-800 p-2 rounded-md">
                      <p className="text-sm font-mono">Suggested fix: {bug.fix}</p>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="p-4 text-center text-green-400">
                No security issues found! Your code looks good.
              </div>
            )}
          </div>
        )}
        
        {loadingContents && (
          <div className="mt-4 p-4 flex justify-center">
            <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue-500"></div>
          </div>
        )}
        
        {contentsResult && contentsResult.contents && (
          <div className="mt-4 bg-gray-700 rounded-md overflow-hidden">
            <div className="bg-gray-800 p-3 border-b border-gray-600 flex items-center justify-between">
              <div className="flex items-center">
                <button 
                  onClick={navigateUp}
                  disabled={!currentPath}
                  className="mr-2 p-1 rounded hover:bg-gray-700 disabled:opacity-50"
                >
                  ↑ Up
                </button>
                <span className="text-sm font-mono">
                  /{currentPath}
                </span>
              </div>
            </div>
            
            <div className="divide-y divide-gray-600">
              {contentsResult.contents.map((item) => (
                <div 
                  key={item.path}
                  className="p-3 hover:bg-gray-600 flex items-center cursor-pointer"
                  onClick={() => item.content_type === 'dir' ? navigateToPath(item.path) : window.open(item.html_url, '_blank')}
                >
                  <div className="w-6 h-6 mr-3 relative">
                    <Image
                      src={getFileIcon(item.content_type)}
                      alt={`${item.content_type === 'dir' ? 'Directory' : 'File'} icon for ${item.name}`}
                      fill
                      style={{ objectFit: 'contain' }}
                    />
                  </div>
                  <div className="flex-1">
                    <div className="font-medium">{item.name}</div>
                    {item.content_type === 'file' && (
                      <div className="text-xs text-gray-400">{formatFileSize(item.size)}</div>
                    )}
                  </div>
                  {item.content_type === 'dir' && (
                    <div className="text-blue-400">→</div>
                  )}
                </div>
              ))}
              
              {contentsResult.contents.length === 0 && (
                <div className="p-4 text-center text-gray-400">
                  This directory is empty
                </div>
              )}
            </div>
          </div>
        )}
        
        {contentsResult && contentsResult.file_content && (
          <div className="mt-4 bg-gray-700 rounded-md overflow-hidden">
            <div className="bg-gray-800 p-3 border-b border-gray-600">
              <div className="font-medium">{contentsResult.file_content.name}</div>
              <div className="text-xs text-gray-400">{formatFileSize(contentsResult.file_content.size)}</div>
            </div>
            
            {contentsResult.file_content.content ? (
              <pre className="p-4 overflow-x-auto text-sm font-mono">
                {contentsResult.file_content.content}
              </pre>
            ) : (
              <div className="p-4 text-center text-gray-400">
                Content not available for preview
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
