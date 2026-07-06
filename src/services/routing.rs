//! Per-app domain model and Traefik label injection.

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use serde_yaml::{Mapping, Value};

use super::traefik::NETWORK_NAME;

/// Routing specification for one compose service.
#[derive(Debug, Clone)]
pub struct DomainSpec {
    /// Compose service name (also used as Traefik router name).
    pub service: String,
    /// Public hostname, e.g. `app.example.com` or `*.example.com` for wildcard.
    pub host: String,
    /// Internal container port Traefik should forward to.
    pub port: u16,
    /// Optional path prefix (default `/`).
    pub path: Option<String>,
    /// Enable TLS via Let's Encrypt resolver `le`.
    pub https: bool,
}

impl DomainSpec {
    fn router(&self) -> String {
        self.service.replace(['.', '/', ' '], "-")
    }

    pub fn is_wildcard(&self) -> bool {
        self.host.trim().starts_with("*.")
    }

    /// Generate Traefik Docker provider labels for this service.
    pub fn labels(&self) -> Vec<String> {
        let r = self.router();
        let entrypoint = if self.https { "websecure" } else { "web" };
        let host = self.host.trim();

        let rule = if self.is_wildcard() {
            let base = host.trim_start_matches("*.");
            format!("HostRegexp(`{{subdomain:.+}}.{base}`)")
        } else {
            match &self.path {
                Some(p) if p != "/" => format!("Host(`{host}`) && PathPrefix(`{p}`)"),
                _ => format!("Host(`{host}`)"),
            }
        };

        let mut labels = vec![
            "traefik.enable=true".to_string(),
            format!("traefik.docker.network={NETWORK_NAME}"),
            format!("traefik.http.routers.{r}.rule={rule}"),
            format!("traefik.http.routers.{r}.entrypoints={entrypoint}"),
            format!("traefik.http.routers.{r}.service={r}"),
            format!(
                "traefik.http.services.{r}.loadbalancer.server.port={}",
                self.port
            ),
        ];

        if self.https {
            labels.push(format!("traefik.http.routers.{r}.tls=true"));
            labels.push(format!("traefik.http.routers.{r}.tls.certresolver=le"));
            if self.is_wildcard() {
                let base = host.trim_start_matches("*.");
                labels.push(format!("traefik.http.routers.{r}.tls.domains[0].main={base}"));
                labels.push(format!("traefik.http.routers.{r}.tls.domains[0].sans={host}"));
            }
        }

        labels
    }
}

/// Validate domain/port/service before deploy.
pub fn validate_domain_spec(spec: &DomainSpec) -> Result<()> {
    let host = spec.host.trim();
    if host.is_empty() {
        bail!("domain cannot be empty when routing is enabled");
    }
    if host.len() > 253 {
        bail!("domain is too long");
    }
    if !host
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '*')
    {
        bail!("domain contains invalid characters (use letters, digits, dots, hyphens, *)");
    }
    if host.starts_with('*') && !host.starts_with("*.") {
        bail!("wildcard domain must start with *., e.g. *.example.com");
    }
    if spec.port == 0 || spec.port > 65535 {
        bail!("port must be between 1 and 65535");
    }
    let service = spec.service.trim();
    if service.is_empty() {
        bail!("compose service name is required");
    }
    Ok(())
}

/// Patch user compose: merge Traefik labels and attach to `doktui-network`.
pub fn inject_routing(compose_yaml: &str, spec: &DomainSpec) -> Result<String> {
    let mut doc: Value = serde_yaml::from_str(compose_yaml)?;
    let root = doc
        .as_mapping_mut()
        .context("compose root must be a mapping")?;

    let services = root
        .get_mut(Value::from("services"))
        .and_then(Value::as_mapping_mut)
        .context("compose must contain a `services` mapping")?;

    let svc = services
        .get_mut(Value::from(spec.service.clone()))
        .and_then(Value::as_mapping_mut)
        .with_context(|| format!("service `{}` not found in compose", spec.service))?;

    let existing_labels = svc.get(&Value::from("labels"));
    svc.insert(
        Value::from("labels"),
        merge_labels(existing_labels, &spec.labels()),
    );

    let existing_networks = svc.get(&Value::from("networks"));
    svc.insert(
        Value::from("networks"),
        merge_service_networks(existing_networks, NETWORK_NAME),
    );

    let existing_root_networks = root.get(&Value::from("networks"));
    root.insert(
        Value::from("networks"),
        merge_root_networks(existing_root_networks, NETWORK_NAME),
    );

    Ok(serde_yaml::to_string(&doc)?)
}

fn merge_labels(existing: Option<&Value>, new_labels: &[String]) -> Value {
    let mut map: HashMap<String, String> = HashMap::new();

    if let Some(Value::Sequence(seq)) = existing {
        for item in seq {
            if let Value::String(raw) = item {
                if let Some((key, value)) = raw.split_once('=') {
                    map.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    for label in new_labels {
        if let Some((key, value)) = label.split_once('=') {
            map.insert(key.to_string(), value.to_string());
        }
    }

    let mut pairs: Vec<_> = map.into_iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    Value::Sequence(
        pairs
            .into_iter()
            .map(|(k, v)| Value::String(format!("{k}={v}")))
            .collect(),
    )
}

fn merge_service_networks(existing: Option<&Value>, network: &str) -> Value {
    let mut names = Vec::new();

    if let Some(Value::Sequence(seq)) = existing {
        for item in seq {
            if let Value::String(name) = item {
                names.push(name.clone());
            }
        }
    } else if let Some(Value::Mapping(map)) = existing {
        for key in map.keys() {
            if let Value::String(name) = key {
                names.push(name.clone());
            }
        }
    }

    if !names.iter().any(|n| n == network) {
        names.push(network.to_string());
    }

    Value::Sequence(names.into_iter().map(Value::String).collect())
}

fn merge_root_networks(existing: Option<&Value>, network: &str) -> Value {
    let mut map = if let Some(Value::Mapping(m)) = existing {
        m.clone()
    } else {
        Mapping::new()
    };

    if !map.contains_key(Value::from(network)) {
        let mut ext = Mapping::new();
        ext.insert(Value::from("external"), Value::from(true));
        map.insert(Value::from(network), Value::Mapping(ext));
    }

    Value::Mapping(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"services:
  app:
    image: nginx:alpine
    restart: unless-stopped
"#;

    const MULTI_SERVICE: &str = r#"services:
  app:
    image: nginx:alpine
    networks:
      - default
    labels:
      - "com.example.team=platform"
  db:
    image: postgres:16
    networks:
      - default
networks:
  default:
    driver: bridge
"#;

    #[test]
    fn https_labels_include_certresolver() {
        let spec = DomainSpec {
            service: "app".into(),
            host: "app.example.com".into(),
            port: 3000,
            path: None,
            https: true,
        };
        let labels = spec.labels();
        assert!(labels.iter().any(|l| l.contains("Host(`app.example.com`)")));
        assert!(
            labels
                .iter()
                .any(|l| l == "traefik.http.services.app.loadbalancer.server.port=3000")
        );
        assert!(labels.iter().any(|l| l.contains("certresolver=le")));
    }

    #[test]
    fn path_prefix_added_when_not_root() {
        let spec = DomainSpec {
            service: "api".into(),
            host: "example.com".into(),
            port: 8080,
            path: Some("/api".into()),
            https: false,
        };
        assert!(
            spec.labels()
                .iter()
                .any(|l| l.contains("PathPrefix(`/api`)"))
        );
    }

    #[test]
    fn inject_routing_adds_network_and_labels() {
        let spec = DomainSpec {
            service: "app".into(),
            host: "whoami.local".into(),
            port: 80,
            path: None,
            https: true,
        };
        let out = inject_routing(SAMPLE, &spec).unwrap();
        assert!(out.contains("doktui-network"));
        assert!(out.contains("traefik.enable"));
        assert!(out.contains("external: true"));
    }

    #[test]
    fn inject_routing_merges_existing_networks_and_labels() {
        let spec = DomainSpec {
            service: "app".into(),
            host: "app.example.com".into(),
            port: 3000,
            path: None,
            https: true,
        };
        let out = inject_routing(MULTI_SERVICE, &spec).unwrap();
        assert!(out.contains("com.example.team=platform"));
        assert!(out.contains("doktui-network"));
        assert!(out.contains("default:"));

        let doc: Value = serde_yaml::from_str(&out).unwrap();
        let app = doc["services"]["app"].as_mapping().unwrap();
        let networks = app["networks"].as_sequence().unwrap();
        let names: Vec<_> = networks
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        assert!(names.contains(&"default".to_string()));
        assert!(names.contains(&NETWORK_NAME.to_string()));

        // db service untouched
        assert!(doc["services"]["db"].is_mapping());
        let db_networks = doc["services"]["db"]["networks"].as_sequence().unwrap();
        assert_eq!(db_networks.len(), 1);
    }

    #[test]
    fn inject_routing_errors_on_missing_service() {
        let spec = DomainSpec {
            service: "missing".into(),
            host: "x.com".into(),
            port: 80,
            path: None,
            https: false,
        };
        assert!(inject_routing(SAMPLE, &spec).is_err());
    }

    #[test]
    fn wildcard_labels_include_sans() {
        let spec = DomainSpec {
            service: "app".into(),
            host: "*.example.com".into(),
            port: 443,
            path: None,
            https: true,
        };
        let labels = spec.labels();
        assert!(labels.iter().any(|l| l.contains("HostRegexp")));
        assert!(labels.iter().any(|l| l.contains("sans=*.example.com")));
    }

    #[test]
    fn validate_rejects_bad_port_and_host() {
        let spec = DomainSpec {
            service: "app".into(),
            host: "bad host".into(),
            port: 0,
            path: None,
            https: false,
        };
        assert!(validate_domain_spec(&spec).is_err());
    }
}
