//! GitHub REST API client (OAuth Device Flow access tokens).

use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct GitHubUser {
    pub login: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GitHubRepo {
    pub full_name: String,
    pub owner: String,
    pub name: String,
    pub default_branch: String,
    pub private: bool,
}

#[derive(Debug, Deserialize)]
struct ApiRepo {
    full_name: String,
    name: String,
    private: bool,
    default_branch: String,
    owner: ApiOwner,
}

#[derive(Debug, Deserialize)]
struct ApiOwner {
    login: String,
}

#[derive(Debug, Deserialize)]
struct ApiBranch {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ApiCommitRef {
    sha: String,
}

fn client(token: &str) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("doktui")
        .default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            h.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {token}")
                    .parse()
                    .context("invalid GitHub OAuth token for Authorization header")?,
            );
            h.insert(
                reqwest::header::ACCEPT,
                "application/vnd.github+json".parse().unwrap(),
            );
            h.insert("X-GitHub-Api-Version", "2022-11-28".parse().unwrap());
            h
        })
        .build()
        .context("failed to build GitHub HTTP client")
}

async fn check_response(resp: reqwest::Response) -> Result<reqwest::Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let body = resp.text().await.unwrap_or_default();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        bail!("GitHub auth failed ({status}) — reconnect the account under Git Providers");
    }
    bail!("GitHub API error ({status}): {}", body.trim());
}

#[derive(Debug, Deserialize)]
struct ApiUser {
    login: String,
    name: Option<String>,
}

/// Authenticated user profile.
pub async fn fetch_user(token: &str) -> Result<GitHubUser> {
    let client = client(token)?;
    let resp = client
        .get("https://api.github.com/user")
        .send()
        .await
        .context("failed to reach api.github.com")?;
    let resp = check_response(resp).await?;
    let user: ApiUser = resp.json().await.context("invalid user JSON")?;
    Ok(GitHubUser {
        login: user.login,
        name: user.name,
    })
}

/// List repositories visible to the authenticated user (first page, up to 100).
pub async fn list_repos(token: &str) -> Result<Vec<GitHubRepo>> {
    let client = client(token)?;
    let resp = client
        .get("https://api.github.com/user/repos")
        .query(&[
            ("per_page", "100"),
            ("sort", "updated"),
            ("affiliation", "owner,collaborator,organization_member"),
        ])
        .send()
        .await
        .context("failed to reach api.github.com")?;
    let resp = check_response(resp).await?;
    let repos: Vec<ApiRepo> = resp.json().await.context("invalid repos JSON")?;
    Ok(repos
        .into_iter()
        .map(|r| GitHubRepo {
            full_name: r.full_name,
            owner: r.owner.login,
            name: r.name,
            default_branch: r.default_branch,
            private: r.private,
        })
        .collect())
}

pub async fn list_branches(token: &str, owner: &str, repo: &str) -> Result<Vec<String>> {
    let client = client(token)?;
    let url = format!("https://api.github.com/repos/{owner}/{repo}/branches");
    let resp = client
        .get(&url)
        .query(&[("per_page", "100")])
        .send()
        .await
        .context("failed to reach api.github.com")?;
    let resp = check_response(resp).await?;
    let branches: Vec<ApiBranch> = resp.json().await.context("invalid branches JSON")?;
    Ok(branches.into_iter().map(|b| b.name).collect())
}

pub async fn latest_commit_sha(
    token: &str,
    owner: &str,
    repo: &str,
    branch: &str,
) -> Result<String> {
    let client = client(token)?;
    let url = format!("https://api.github.com/repos/{owner}/{repo}/commits/{branch}");
    let resp = client
        .get(&url)
        .send()
        .await
        .context("failed to reach api.github.com")?;
    let resp = check_response(resp).await?;
    let commit: ApiCommitRef = resp.json().await.context("invalid commit JSON")?;
    Ok(commit.sha)
}

/// Build a public HTTPS clone URL (no credentials).
pub fn public_clone_url(owner: &str, repo: &str) -> String {
    format!("https://github.com/{owner}/{repo}.git")
}

/// Authenticated clone URL. Caller must scrub remotes after use.
pub fn authed_clone_url(owner: &str, repo: &str, token: &str) -> String {
    format!("https://x-access-token:{token}@github.com/{owner}/{repo}.git")
}

/// Redact tokens from error / log strings.
pub fn redact_secrets(text: &str, token: &str) -> String {
    if token.is_empty() {
        return text.to_string();
    }
    text.replace(token, "***")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_url_shape() {
        assert_eq!(
            public_clone_url("doklabs", "doktui"),
            "https://github.com/doklabs/doktui.git"
        );
    }

    #[test]
    fn authed_url_embeds_token() {
        let url = authed_clone_url("o", "r", "ghp_secret");
        assert!(url.contains("x-access-token:ghp_secret@"));
        assert!(url.ends_with("github.com/o/r.git"));
    }

    #[test]
    fn redact_secrets_strips_token() {
        let s = redact_secrets("clone failed: https://x-access-token:abc123@github.com/x/y", "abc123");
        assert!(!s.contains("abc123"));
        assert!(s.contains("***"));
    }
}
