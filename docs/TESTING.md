# Testing DokTUI

DokTUI manages real servers over SSH. You do **not** need a remote VPS to develop or test most of it. This guide shows how to test locally, from pure unit tests to a local SSH + Docker environment.

## Test levels

| Level | Needs SSH | Needs Docker | Use case |
|---|---|---|---|
| **Unit tests** | No | No | Pure logic, keybindings, routing labels, theme resolution, cron parsing |
| **Mocked service tests** | No | No | Test `DockerController` / `TraefikProvisioner` command generation without real SSH |
| **Local SSH container** | Yes (localhost) | Yes (host socket or DinD) | Manual smoke test / integration test of the full app |
| **CI integration** | Yes | Yes | GitHub Actions with `services:` or `testcontainers` |

## 1. Unit tests — `cargo test`

The default test suite does not connect to SSH and does not run Docker:

```sh
cargo test --all-targets
```

It already covers:

- Keybinding mapping (`app::map_key_tests`)
- Theme resolution and validation
- Sprite rendering
- Traefik label generation and Docker Compose injection
- Cron expression validation
- Backoff logic
- i18n formatting

All of these are stateless and safe to run on any machine.

### Running a single test with logs

```sh
RUST_LOG=debug cargo test --all-targets test_name -- --nocapture
```

Use `RUST_BACKTRACE=1` if a test panics.

## 2. Mocking the SSH layer for service tests

The services (`DockerController`, `TraefikProvisioner`, `RemoteProvisioner`) already accept `&mut dyn SshBackend` instead of a concrete `SshSession`. The `SshBackend` trait is in `src/services/ssh/mod.rs`:

```rust
#[async_trait]
pub trait SshBackend: Send {
    async fn exec(&mut self, command: &str) -> Result<CommandOutput>;
    async fn write_remote_file(&mut self, remote_path: &str, content: &[u8]) -> Result<()>;
}
```

In tests, implement a fake backend:

```rust
use std::collections::HashMap;
use async_trait::async_trait;
use crate::services::ssh::{CommandOutput, SshBackend};

#[derive(Default)]
struct MockSshBackend {
    commands: Vec<String>,
    responses: HashMap<String, CommandOutput>,
}

#[async_trait]
impl SshBackend for MockSshBackend {
    async fn exec(&mut self, command: &str) -> Result<CommandOutput> {
        self.commands.push(command.to_string());
        Ok(self.responses.get(command).cloned().unwrap_or(CommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
        }))
    }

    async fn write_remote_file(&mut self, _remote_path: &str, _content: &[u8]) -> Result<()> {
        // optionally record the file
        Ok(())
    }
}
```

Then test `DockerController` without a real SSH server:

```rust
#[tokio::test]
async fn probe_detects_docker() {
    let mut backend = MockSshBackend {
        responses: HashMap::from([(
            "command -v docker".to_string(),
            CommandOutput {
                stdout: "/usr/bin/docker\n".to_string(),
                stderr: String::new(),
                exit_code: 0,
            },
        )]),
        ..Default::default()
    };

    assert!(DockerController::probe(&mut backend).await.unwrap());
    assert_eq!(backend.commands, vec!["command -v docker"]);
}
```

A working implementation of `MockSshBackend` is in `src/services/docker.rs` under `#[cfg(test)]`.

This lets you assert the exact command strings sent to the server, without ever opening a socket.

## 3. Local SSH + Docker environment

For a real smoke test, run a local SSH server that also has Docker access. This is the closest to production without a remote VPS.

### Option A: Docker container with host Docker socket (quickest)

Create a test Dockerfile at `scripts/test-server/Dockerfile`:

```dockerfile
FROM alpine:latest

RUN apk add --no-cache openssh-server docker-cli docker-compose curl
RUN ssh-keygen -A

# Create a root user authorized_keys file
RUN mkdir -p /root/.ssh && chmod 700 /root/.ssh
ARG AUTHORIZED_KEY
RUN echo "$AUTHORIZED_KEY" > /root/.ssh/authorized_keys && chmod 600 /root/.ssh/authorized_keys

EXPOSE 22
CMD ["/usr/sbin/sshd", "-D"]
```

Build and run:

```sh
# Get the public key DokTUI will use for the server
PUBKEY="$(cat ~/.local/share/doktui/doktui_key.pub 2>/dev/null || cat ~/.ssh/doktui_key.pub 2>/dev/null)"

# Build
docker build --build-arg AUTHORIZED_KEY="$PUBKEY" -t doktui-test-server -f scripts/test-server/Dockerfile .

# Run with access to the host Docker daemon
docker run -d --name doktui-test \
  -p 2222:22 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  --restart unless-stopped \
  doktui-test-server
```

Then register in DokTUI:

- **Host:** `localhost`
- **Port:** `2222`
- **User:** `root`
- **ACME email:** your email
- Add the public key to the container when you build it (as shown above).

### Option B: Docker-in-Docker (most isolated)

If you do not want to mount the host Docker socket, use a fully isolated Docker daemon:

```sh
# Run a DinD container with SSH access
docker run -d --privileged --name doktui-dind \
  -p 2222:22 \
  docker:27-dind

# Install SSH and add your key inside the container
docker exec doktui-dind sh -c "
  apk add --no-cache openssh-server docker-cli docker-compose
  ssh-keygen -A
  mkdir -p /root/.ssh
  echo '$(cat ~/.local/share/doktui/doktui_key.pub)' > /root/.ssh/authorized_keys
  chmod 600 /root/.ssh/authorized_keys
  /usr/sbin/sshd -D &
"
```

This is heavier but gives a clean environment. Great for CI.

### Option C: WSL2 + Docker (Windows)

If you use Docker Desktop with WSL2 backend:

1. Open a WSL2 distro.
2. Install and start SSH:
   ```sh
   sudo apt update
   sudo apt install -y openssh-server
   sudo service ssh start
   ```
3. Add your DokTUI public key to `~/.ssh/authorized_keys`.
4. Get the WSL2 IP:
   ```sh
   ip addr show eth0
   ```
5. Register that IP in DokTUI, port `22`, user your WSL username.

This is the most convenient setup for daily Windows development.

## 4. Config and data isolation in tests

DokTUI stores config and keys in OS-specific directories:

- Linux/WSL: `~/.config/doktui/` and `~/.local/share/doktui/`
- macOS: `~/Library/Application Support/doktui/` and `~/Library/Preferences/doktui/`
- Windows: `%APPDATA%\doktui\` and `%LOCALAPPDATA%\doktui\`

To avoid polluting your real config while testing:

- **Unit tests:** construct `AppState` directly with `AppState::new(...)`. Do not call `bootstrap()` in tests unless you intend to touch the real filesystem.
- **Integration tests:** run in a clean environment (WSL2 distro, container, CI runner) or set:
  - `XDG_CONFIG_HOME=/tmp/doktui-test/config`
  - `XDG_DATA_HOME=/tmp/doktui-test/data`

  on Linux/WSL before starting `doktui`. Then `directories` will use those paths.

## 5. CI integration tests

You can run a full SSH + Docker environment in GitHub Actions:

```yaml
jobs:
  integration:
    runs-on: ubuntu-latest
    services:
      ssh-dind:
        image: docker:27-dind
        ports:
          - 2222:22
        options: --privileged
    steps:
      - uses: actions/checkout@v4
      - run: |
          # Wait for DinD, install openssh and key inside the service
          docker run --rm --network host alpine:edge sh -c \
            "apk add --no-cache openssh-client && ssh-keygen -t ed25519 -N '' -f /tmp/doktui_key"
          # ... copy key, install sshd, and run integration tests
      - run: cargo test --all-targets
```

For a more maintainable CI, consider adding a `testcontainers` based harness or a custom `Dockerfile` built in the workflow.

## 6. Debugging tests

- `RUST_LOG=debug cargo test` — see tracing logs.
- `RUST_LOG=doktui=trace` — see only DokTUI logs.
- `RUST_BACKTRACE=1` — full backtrace on panic.
- `cargo test -- --nocapture` — see `println!` output.
- `cargo test -- --test-threads=1` — run tests sequentially.

## Recommended workflow

1. **Every code change:** `cargo test --all-targets` (no SSH needed).
2. **New service logic:** add a `MockSshBackend` test that asserts the right SSH commands are sent.
3. **Before a release / major refactor:** do a manual smoke test with a local SSH container (Option A or B).
4. **CI:** run unit tests in the workflow; keep integration tests behind a feature flag or nightly job until they are stable.

## See also

- [`DEVELOPMENT.md`](DEVELOPMENT.md) — day-to-day dev commands
- [`PROJECT-GUIDE.md`](PROJECT-GUIDE.md) — architecture and tech stack
- [`AGENTS.md`](../AGENTS.md) — verification commands used by Devin
