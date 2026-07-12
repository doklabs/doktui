# Traefik Routing — Problem Analysis & Solution Design

**Status:** Implemented (Phase 1)
**Date:** July 6, 2026
**Project:** Doklabs — DokTUI (open source)
**Reference:** [PRD.md](./PRD.md) §7.1 (Domain & routing), [TDD.md](./TDD.md) §8 (Deploy & Container Management)

---

## 1. Executive Summary

This document captures the original problem with DokTUI's Traefik routing and the solution that was implemented. In its earlier state, DokTUI **could not actually route traffic to deployed apps**. The root cause was two fundamentally missing pieces:

1. **No Traefik label generation** — deploy simply uploaded the user's raw compose.
2. **No shared network** between Traefik and the app containers — so Traefik could not reach the service on the network, even with hand-written labels.

Both are now fixed. This document explains the problem (grounded in code), how Dokploy solves the same problem, and the implemented solution with concrete Rust.

---

## 2. Prior State (the problem)

### 2.1 Traefik install (`services/traefik.rs`)

Traefik was installed via a static compose:

- Image `traefik:v3.7`, container `doktui-traefik`, ports 80/443.
- Docker provider with `exposedbydefault=false`.
- A single ACME resolver `le` (HTTP-01 challenge), with a **hardcoded** email `admin@example.com`.
- A `traefik-certs` volume for `acme.json`.

### 2.2 App deploy (`services/docker.rs` → `deploy_compose`)

Deploy flow:

1. `mkdir` remote dir.
2. Write `docker-compose.yml` from the user-supplied string (base64 → file), **unmodified**.
3. Write `.env` if present.
4. `docker compose up -d`.

`Message::SubmitDeploy { server_id, remote_dir, compose }` had no domain, port, or TLS field. Nothing touched Traefik labels or the network.

---

## 3. Problem List (grounded in code)

| # | Problem | Impact | Evidence |
|---|---------|--------|----------|
| 1 | **No Traefik label generation** at deploy | App not routed unless the user writes all labels by hand | `deploy_compose` uploaded compose verbatim |
| 2 | **No shared network** Traefik ↔ app | Traefik cannot reach the app container on the network | App compose created its own project network; no `external` network |
| 3 | **Hardcoded ACME email** `admin@example.com` | Let's Encrypt notices/expiry go to a fake address; rate-limit risk | `traefik.rs` compose constant |
| 4 | **HTTP-01 challenge only** | No wildcard certs; fails when :80 isn't publicly reachable | `--certificatesresolvers.le.acme.httpchallenge` |
| 5 | **No HTTP→HTTPS redirect** | `http://` traffic not forced to TLS | No redirection config on the `web` entrypoint |
| 6 | **No per-app domain model** in state/config | No source of truth for domain, port, TLS per app | `Message::SubmitDeploy` without a domain field |
| 7 | **Provider not scoped to a network** | Traefik may pick the wrong container IP on multi-network setups | No `--providers.docker.network` |

Problems #1 and #2 were **blockers** — without both, routing didn't work at all. The rest are production quality/reliability issues.

---

## 4. How Dokploy Solves It (reference)

From `packages/server/src/utils/docker/domain.ts` and `setup/traefik-setup.ts`:

- A **shared external network** named `dokploy-network`. Traefik and every app service attach to it, so Traefik can always reach the app.
- **Automatic label generation** per domain, following the Traefik v3 pattern:

  ```
  traefik.http.routers.<router>.rule=Host(`<host>`) [&& PathPrefix(`<path>`)]
  traefik.http.routers.<router>.entrypoints=<entrypoint>
  traefik.http.services.<router>.loadbalancer.server.port=<port>
  traefik.http.routers.<router>.service=<router>
  traefik.http.routers.<router>.tls.certresolver=letsencrypt   # for HTTPS
  traefik.http.routers.<router>.tls=true
  ```

DokTUI adopts the same pattern, simplified for the "runs locally" model.

---

## 5. Solution Architecture

Four pillars:

1. **A shared `doktui-network`** (external, created at provisioning). Traefik and each app service attach to it.
2. **A per-app domain model** (`DomainSpec`) as the source of truth: host, container port, https, path, entrypoint.
3. **Automatic label injection** into the compose at deploy time, based on `DomainSpec` — the user only fills in domain & port, not raw labels.
4. **A mature Traefik config**: configurable ACME email, a global HTTP→HTTPS redirect, a pinned `providers.docker.network`, and DNS-01 for wildcards.

```
┌─────────────────┐        doktui-network (external, shared)        ┌──────────────┐
│  doktui-traefik │◄───────────────────────────────────────────────►│  app-service │
│  :80 / :443     │   label: Host(app.example.com) → port 3000       │  (container) │
└─────────────────┘                                                  └──────────────┘
        ▲  static config: entrypoints, ACME(email), redirect, provider.network
```

---

## 6. Implementation

### 6.1 Improved Traefik static config (`services/traefik.rs`)

The Traefik compose takes the email as a parameter, adds a global redirect, and pins the provider network:

```rust
/// Build Traefik compose with the correct ACME email & HTTP→HTTPS redirect.
pub fn traefik_compose(acme_email: &str) -> String {
    format!(
        r#"services:
  traefik:
    image: traefik:v3.7
    container_name: doktui-traefik
    restart: unless-stopped
    networks:
      - doktui-network
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - traefik-certs:/letsencrypt
    command:
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--providers.docker.network=doktui-network"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
      # Global HTTP → HTTPS redirect
      - "--entrypoints.web.http.redirections.entrypoint.to=websecure"
      - "--entrypoints.web.http.redirections.entrypoint.scheme=https"
      # ACME (Let's Encrypt) via HTTP-01
      - "--certificatesresolvers.le.acme.httpchallenge=true"
      - "--certificatesresolvers.le.acme.httpchallenge.entrypoint=web"
      - "--certificatesresolvers.le.acme.email={acme_email}"
      - "--certificatesresolvers.le.acme.storage=/letsencrypt/acme.json"
networks:
  doktui-network:
    external: true
volumes:
  traefik-certs:
"#
    )
}
```

Create the network before bringing Traefik up (idempotent):

```rust
/// Create the shared network if it doesn't exist. Safe to call repeatedly.
pub async fn ensure_network(session: &mut SshSession) -> Result<()> {
    let exists = session
        .exec("docker network ls --filter name=^doktui-network$ --format '{{.Name}}'")
        .await?;
    if !exists.stdout.contains("doktui-network") {
        let out = session.exec("docker network create doktui-network").await?;
        if out.exit_code != 0 {
            bail!("failed to create doktui-network: {}", out.stderr.trim());
        }
    }
    Ok(())
}
```

`RemoteProvisioner::run` calls `ensure_network` **before** installing Traefik.

### 6.2 Domain model (`services/routing.rs`)

```rust
//! Per-app domain model and Traefik label generation.

/// Routing spec for one app.
#[derive(Debug, Clone)]
pub struct DomainSpec {
    /// Compose service name (used as the Traefik router name).
    pub service: String,
    /// Public host, e.g. "app.example.com".
    pub host: String,
    /// Internal container port served.
    pub port: u16,
    /// Optional path (default "/").
    pub path: Option<String>,
    /// Enable TLS (Let's Encrypt via resolver "le").
    pub https: bool,
}

impl DomainSpec {
    /// A stable, safe Traefik router/service name.
    fn router(&self) -> String {
        self.service.replace(['.', '/', ' '], "-")
    }

    /// Produce Traefik labels for this service.
    pub fn labels(&self) -> Vec<String> {
        let r = self.router();
        let entrypoint = if self.https { "websecure" } else { "web" };

        let rule = match &self.path {
            Some(p) if p != "/" => format!("Host(`{}`) && PathPrefix(`{}`)", self.host, p),
            _ => format!("Host(`{}`)", self.host),
        };

        let mut labels = vec![
            "traefik.enable=true".to_string(),
            "traefik.docker.network=doktui-network".to_string(),
            format!("traefik.http.routers.{r}.rule={rule}"),
            format!("traefik.http.routers.{r}.entrypoints={entrypoint}"),
            format!("traefik.http.routers.{r}.service={r}"),
            format!("traefik.http.services.{r}.loadbalancer.server.port={}", self.port),
        ];

        if self.https {
            labels.push(format!("traefik.http.routers.{r}.tls=true"));
            labels.push(format!("traefik.http.routers.{r}.tls.certresolver=le"));
        }

        labels
    }
}
```

### 6.3 Injecting labels & the network into compose

The user's compose is patched: (a) add labels to the target service, (b) attach the service to `doktui-network`, (c) declare the network as `external`. Safe YAML editing uses `serde_yaml`.

**Important:** labels and networks are **merged**, not replaced — any labels the user already wrote and any networks the service already belongs to are preserved (so app↔db connectivity is not broken).

```rust
use serde_yaml::{Mapping, Value};

/// Patch compose: merge Traefik labels + attach to doktui-network.
pub fn inject_routing(compose_yaml: &str, spec: &DomainSpec) -> anyhow::Result<String> {
    let mut doc: Value = serde_yaml::from_str(compose_yaml)?;
    let root = doc.as_mapping_mut().context("compose is not a mapping")?;

    let services = root
        .get_mut(Value::from("services"))
        .and_then(Value::as_mapping_mut)
        .context("compose has no `services`")?;
    let svc = services
        .get_mut(Value::from(spec.service.clone()))
        .and_then(Value::as_mapping_mut)
        .with_context(|| format!("service `{}` not found", spec.service))?;

    // Merge (never clobber) existing labels & networks.
    let existing_labels = svc.get(&Value::from("labels"));
    svc.insert(Value::from("labels"), merge_labels(existing_labels, &spec.labels()));

    let existing_networks = svc.get(&Value::from("networks"));
    svc.insert(Value::from("networks"), merge_service_networks(existing_networks, "doktui-network"));

    let existing_root = root.get(&Value::from("networks"));
    root.insert(Value::from("networks"), merge_root_networks(existing_root, "doktui-network"));

    Ok(serde_yaml::to_string(&doc)?)
}
```

### 6.4 Updated deploy flow

`Message::SubmitDeploy` is extended with a domain field:

```rust
SubmitDeploy {
    server_id: Uuid,
    remote_dir: String,
    compose: String,
    domain: Option<DomainSpec>, // optional — deploy without a domain is still allowed
}
```

In the deploy handler:

```rust
let final_compose = match &domain {
    Some(spec) => routing::inject_routing(&compose, spec)?,
    None => compose,
};
DockerController::deploy_compose(&mut session, &remote_dir, &final_compose, &env).await?;
```

The user fills in only **host + port + an HTTPS toggle**; DokTUI assembles the labels and network. This aligns with the PRD's "keep it simple" principle.

---

## 7. Legacy Traefik Migration

A server provisioned before the shared-network change runs an old Traefik that isn't on `doktui-network`, so new routing silently fails. To handle this, `TraefikProvisioner::status` classifies the install as:

- `NotRunning` — no Traefik container.
- `Legacy` — running but not on `doktui-network` / missing the provider-network config.
- `Healthy` — running on `doktui-network` with the expected config.

When `Legacy` is detected, provisioning **auto-migrates** (recreates Traefik with the new config) instead of requiring a manual reinstall.

---

## 8. Rollout Phases

- **Phase 1 (blocker, done):** shared network + label injection + configurable ACME email + legacy migration. Routing now works end-to-end.
- **Phase 2 (reliability):** HTTP→HTTPS redirect (in the static config), domain validation, cert status in the UI, post-deploy verification.
- **Phase 3 (advanced):** DNS-01 challenge for wildcards (Cloudflare), custom cert resolver, middlewares (basic-auth, rate-limit).

---

## 9. Testing

- **Unit:** `DomainSpec::labels()` (http vs https, root vs non-root path), `inject_routing()` (merge idempotency, service-not-found → clear error).
- **Integration:** an SSHD + Docker test container → provision → deploy a sample app (e.g., `traefik/whoami`) with domain `whoami.docker.localhost` → verify Traefik routes it (HTTP 200) and the service joined `doktui-network`.
- **Manual:** deploy to a real VPS with a public domain → verify Let's Encrypt cert issuance & HTTPS redirect.

---

## 10. Open Questions

- Is the ACME email asked once (global per install) or per-server?
- Should DNS-01 support more providers than Cloudflare in the first pass?
- Multiple domains per service in the early phase, or one domain per service to start?
