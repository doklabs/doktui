//! Clone / pull a GitHub repo on the remote host over SSH, then read compose.

use anyhow::{bail, Context, Result};

use super::github::{authed_clone_url, public_clone_url, redact_secrets};
use super::ssh::SshBackend;

/// Escape a string for use inside single-quoted POSIX shell.
pub fn shell_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn join_remote(remote_dir: &str, compose_path: &str) -> String {
    let dir = remote_dir.trim_end_matches('/');
    let path = compose_path.trim_start_matches('/');
    format!("{dir}/{path}")
}

/// Ensure `remote_dir` contains the given GitHub repo on `branch`.
///
/// Uses an authenticated URL only for the duration of clone/fetch, then resets
/// `origin` to the public HTTPS URL so the token is not left in git config.
pub async fn ensure_repo(
    session: &mut dyn SshBackend,
    remote_dir: &str,
    owner: &str,
    repo: &str,
    branch: &str,
    token: Option<&str>,
) -> Result<()> {
    let public = public_clone_url(owner, repo);
    let clone_url = match token {
        Some(t) if !t.is_empty() => authed_clone_url(owner, repo, t),
        _ => public.clone(),
    };
    let q_dir = shell_single_quote(remote_dir);
    let q_branch = shell_single_quote(branch);
    let q_clone = shell_single_quote(&clone_url);
    let q_public = shell_single_quote(&public);

    let probe = session
        .exec(&format!("test -d {q_dir}/.git && echo HAS_GIT || echo NO_GIT"))
        .await?;
    let has_git = probe.stdout.contains("HAS_GIT");

    let script = if has_git {
        format!(
            "set -e
cd {q_dir}
git remote set-url origin {q_clone}
git fetch --depth 1 origin {q_branch}
git checkout -B {q_branch} FETCH_HEAD
git remote set-url origin {q_public}
"
        )
    } else {
        format!(
            "set -e
mkdir -p $(dirname {q_dir})
rm -rf {q_dir}
git clone --depth 1 --branch {q_branch} {q_clone} {q_dir}
cd {q_dir}
git remote set-url origin {q_public}
"
        )
    };

    let out = session.exec(&script).await?;
    if out.exit_code != 0 {
        let err = format!(
            "git sync failed (exit {}): {} {}",
            out.exit_code,
            out.stderr.trim(),
            out.stdout.trim()
        );
        let safe = token.map(|t| redact_secrets(&err, t)).unwrap_or(err);
        bail!("{safe}");
    }
    Ok(())
}

/// Read a compose file from the remote repo working tree.
pub async fn read_compose(
    session: &mut dyn SshBackend,
    remote_dir: &str,
    compose_path: &str,
) -> Result<String> {
    let full = join_remote(remote_dir, compose_path);
    let q = shell_single_quote(&full);
    let out = session
        .exec(&format!("cat {q}"))
        .await
        .context("failed to read compose on remote")?;
    if out.exit_code != 0 {
        bail!(
            "compose not found at {full}: {}",
            out.stderr.trim()
        );
    }
    let body = out.stdout;
    if body.trim().is_empty() {
        bail!("compose file at {full} is empty");
    }
    Ok(body)
}

/// `git pull` + compose up for an existing checkout (auth token for private repos).
#[allow(dead_code)]
pub async fn pull_and_compose_up(
    session: &mut dyn SshBackend,
    remote_dir: &str,
    owner: &str,
    repo: &str,
    branch: &str,
    compose_path: &str,
    token: Option<&str>,
) -> Result<()> {
    ensure_repo(session, remote_dir, owner, repo, branch, token).await?;
    let q_dir = shell_single_quote(remote_dir);
    let compose_arg = if compose_path == "docker-compose.yml" || compose_path.is_empty() {
        String::new()
    } else {
        format!(" -f {}", shell_single_quote(compose_path))
    };
    let out = session
        .exec(&format!(
            "cd {q_dir} && docker compose{compose_arg} pull && docker compose{compose_arg} up -d"
        ))
        .await?;
    if out.exit_code != 0 {
        let err = format!("redeploy failed: {}", out.stderr.trim());
        let safe = token.map(|t| redact_secrets(&err, t)).unwrap_or(err);
        bail!("{safe}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_single_quote_escapes_quotes() {
        assert_eq!(shell_single_quote("a'b"), "'a'\\''b'");
        assert_eq!(shell_single_quote("plain"), "'plain'");
    }

    #[test]
    fn join_remote_normalizes_slashes() {
        assert_eq!(
            join_remote("/opt/app/", "/docker-compose.yml"),
            "/opt/app/docker-compose.yml"
        );
    }
}
