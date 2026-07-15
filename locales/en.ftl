# Brand & chrome
brand-name = DokTUI
brand-subtitle = doklabs
brand-tagline = local TUI for remote deployments
brand-tagline-short = DokTUI · local TUI

# Navigation
nav-home = Home
nav-servers-section = Servers
nav-apps = Apps
nav-monitoring = Monitoring
nav-schedules = Schedules
nav-navigation = NAVIGATION
nav-servers = TARGET
nav-none-yet = (none yet)

# Connection status
conn-online = { $dot } online
conn-connecting = { $dot } connecting
conn-reconnecting = ↻ reconnecting
conn-offline = { $dot } offline

# Footer shortcuts (shell)
shortcut-nav = nav
shortcut-open = open
shortcut-deploy = deploy
shortcut-editor = editor
shortcut-search = search
shortcut-quit = quit
shortcut-continue = continue
shortcut-back = back
shortcut-fps = ~15fps { $spin }

# Sidebar hints
sidebar-tagline = "deploy safely"
sidebar-compact = compact
sidebar-resize-hint = [ ] resize

# Welcome / onboarding
welcome-ssh-title = SSH public key
welcome-ssh-hint = add to ~/.ssh/authorized_keys on your server
welcome-step-register = Register
welcome-step-docker = Check Docker
welcome-step-deploy = Deploy
welcome-btn-add-server = ⏎ Add server
welcome-btn-quit = q Quit
welcome-hint-add = add server ·
welcome-hint-copy = copy key ·
welcome-hint-quit = quit
welcome-footer-copy = [c] copy public key

# Add server form
form-name = Name
form-host = Host
form-port = Port
form-user = User
form-acme-email = ACME email

# Host key
hostkey-trust =
    Host { $host } has an unknown fingerprint:

      { $fingerprint }

    Trust this host? [y/n]
welcome-ssh-box-title = ◆ dedicated ssh key
welcome-ssh-fingerprint = fp:
welcome-ssh-copy = [c] copy
form-add-server-title = Register SSH Server
form-edit-server-title = Edit SSH Server
form-add-server-hint = Tab/Shift+Tab • Enter save • Esc back
hostkey-title = Unknown Host Key
hostkey-none = No host key prompt active

# Shell overlays
search-title = Search (Ctrl+F close, Esc cancel)
error-panel-title = Error detail (Esc close, j/k scroll)
mode-overlay = overlay
mode-compact = compact
mode-sidebar = sidebar
mode-content = content
mode-hint = { $mode } • { $focus } • F6 focus • Ctrl+U mode

# Status bar
status-working = { $spin } working…
status-error-suffix = (E = full error)
status-default-hint = Ctrl+C quit • Ctrl+F search • F6 focus
status-update-available = { $star } { $version } available — run `doktui update`

# Home dashboard
home-title = HOME
home-summary = { $servers } servers · { $apps } apps · { $deploy }
home-deploy-running-0 = 0 deploy running
home-deploy-running-1 = 1 deploy running
home-stat-apps = Apps Online
home-stat-cpu = CPU
home-stat-healthy = Healthy
home-active-server =
    Active server: { $name } @ { $host }:{ $port }
    Status: { $dot } connected

    Use Servers (2) for SSH machines.
    Use Apps (3) for deployments — many apps per server.
home-no-servers = No servers yet.

    Go to Servers (2) → press [a] to register an SSH host.
home-servers-registered =
    { $count } server(s) registered.
    Select a TARGET in the sidebar, then open Apps (3).
home-achievement-label = ACHIEVEMENT
home-achievement-https = Let's Encrypt cert issued · +50 XP
home-deploy-panel = Quick deploy
home-deploy-placeholder = app.example.com
home-deploying-title = { $arrow } DEPLOYING · { $domain }
home-deploy-building = building { $spin }
home-deploy-network-attached = { $check } doktui-network attached
home-deploy-traefik-route = { $check } traefik route → Host(`{ $domain }`) tls:le
home-deploy-pulling = { $arrow } pulling image…

# Servers
servers-title = SSH servers (machines)
servers-shortcut-select = select
servers-shortcut-connect = connect
servers-shortcut-provision = provision
servers-shortcut-add = add
servers-shortcut-edit = edit
servers-shortcut-open = apps
servers-shortcut-remove = remove
servers-shortcut-back = back

# App tools hub (from Apps → t)
deploy-hub-panel-title = App tools
deploy-hub-title = tools for selected server
deploy-hub-hint = These act on the TARGET server. Apps list is under nav [3].
deploy-hub-target = Target server: { $name }
deploy-hub-no-target = Target: (none — pick a server under Servers / sidebar TARGET)
deploy-hub-item-deploy = [d] New app — open canvas (Compose or GitHub)
deploy-hub-item-containers = [c] Containers — start/stop/restart
deploy-hub-item-logs = [l] Logs — stream container output
deploy-hub-item-secrets = [s] Secrets — env vars (encrypted locally)
deploy-hub-item-git = [g] Git Providers — connect GitHub accounts (OAuth)
deploy-hub-item-editor = [e] Editor — edit compose buffer
deploy-hub-shortcut-nav = navigate
deploy-hub-shortcut-open = open

# Deploy form
deploy-title = docker compose + traefik routing
deploy-title-mode = deploy · { $mode }
deploy-mode-compose = Compose paste
deploy-mode-github = GitHub
deploy-mode-hint = Press m to switch Compose ↔ GitHub. Connect GitHub under Git Providers (g) — browser OAuth — then ↑↓ to pick account/repo.
deploy-field-remote-dir = Remote dir
deploy-field-domain = Domain (or *.example.com)
deploy-field-port = Port
deploy-field-service = Service
deploy-field-https = HTTPS (Let's Encrypt)
deploy-field-compose = Compose
deploy-field-account = Account (↑↓)
deploy-field-repo = Repo (↑↓)
deploy-field-branch = Branch
deploy-field-compose-path = Compose path
deploy-field-app-name = App name
deploy-field-auto-deploy = Auto-deploy (poll while open)
deploy-gh-no-account = (connect GitHub under Git Providers)
deploy-gh-pick-account = (↑↓ to pick account)
deploy-gh-no-repos = (press r / Ctrl+R to load repos)
deploy-on = on
deploy-off = off
deploy-compose-editing = (editing — press e for canvas editor)
deploy-compose-lines = { $count } lines
deploy-shortcut-tab = field
deploy-shortcut-https = toggle HTTPS / auto
deploy-shortcut-deploy = deploy
deploy-shortcut-editor = editor
deploy-shortcut-mode = mode
deploy-shortcut-refresh = load repos

# Apps list (top-level nav)
apps-panel-title = Apps
apps-title = applications (many per server)
apps-target-server = Deploy target: { $name } ({ $host }) — ● = on this server
apps-target-none = Deploy target: (none — pick TARGET in sidebar or Servers [2])
apps-empty = No apps yet. Press [n] or Enter for the create wizard (Compose or GitHub). One server can host many apps (different remote dirs).
apps-source-compose = compose
apps-auto-on = auto
apps-auto-off = manual
apps-shortcut-nav = select
apps-shortcut-new = new app
apps-shortcut-open = open canvas
apps-shortcut-redeploy = redeploy
apps-shortcut-tools = server tools
apps-shortcut-delete = delete
apps-shortcut-servers = servers
apps-shortcut-back-list = apps list

# New app wizard (Dokploy-style create flow)
wizard-panel-title = Create service
wizard-step-type = Type
wizard-step-identity = Identity
wizard-step-account = Account
wizard-step-repo = Repository
wizard-type-title = Create Service
wizard-type-subtitle = Choose what to add to this server
wizard-type-hint = Like Dokploy — pick a service type, then name it.
wizard-type-compose = Compose
wizard-type-compose-desc = Paste or edit a docker-compose.yml and deploy over SSH
wizard-type-application = Application
wizard-type-application-desc = Deploy from a GitHub repository (clone/pull on the server)
wizard-identity-title-compose = Create Compose
wizard-identity-title-app = Create Application
wizard-identity-subtitle = Assign a name and description to your service
wizard-field-name = Name
wizard-field-app-name = App name
wizard-field-description = Description
wizard-field-description-placeholder = Description of your service…
wizard-identity-summary = Will open the app canvas · { $mode } · TARGET { $server }. Configure provider, domain, then Deploy.
wizard-account-title = GitHub account
wizard-account-subtitle = Choose which connected account to deploy from
wizard-account-hint = ↑↓ select · Enter continue · c connect another account
wizard-account-empty = No GitHub accounts connected yet.
wizard-account-connect-cta = Connect GitHub (opens Git Providers)
wizard-account-none = (no account)
wizard-repo-title = Repository
wizard-repo-subtitle = Pick a repository from the selected account
wizard-repo-account = Account
wizard-repo-hint = ↑↓ select · r refresh · Enter create
wizard-repo-empty = No repositories loaded. Press r to refresh, or connect an account first.
wizard-create = Create
wizard-shortcut-select = choose
wizard-shortcut-next = continue
wizard-shortcut-cancel = cancel
wizard-shortcut-field = next field
wizard-shortcut-create = create
wizard-shortcut-back = previous step
wizard-shortcut-connect = connect GitHub
wizard-shortcut-refresh = refresh repos
error-wizard-name-required = Name is required
error-git-account-required = Connect GitHub under Git Providers first (opens browser)
status-wizard-compose-created = Compose created — configure provider, then Deploy
status-wizard-app-created = Application created — configure domain, then Deploy

# App canvas (Dokploy-style per-app screen)
canvas-panel-title = App
canvas-draft-name = New app
canvas-new-id = draft
canvas-subtitle = { $mode } · { $dir }
canvas-deploying = deploying…
canvas-ready = ready to deploy
canvas-tab-general = General
canvas-tab-domain = Domain
canvas-tab-env = Env
canvas-tab-deploy = Deploy
canvas-tab-logs = Logs
canvas-general-hint = Provider — Ctrl+M switches Compose ↔ GitHub. Type freely in fields; Tab moves between them.
canvas-domain-hint = Traefik routing — leave domain empty for no public route.
canvas-env-hint = Local secrets injected at deploy. GitHub uses OAuth accounts under Git Providers (not env tokens). Press s to manage.
canvas-env-empty = No secrets yet. Press s to open Secrets.
canvas-deploy-title = deploy summary
canvas-deploy-hint = Ctrl+D deploy · r redeploy (saved apps) · Esc back to Apps
canvas-summary-server = Server: { $value }
canvas-summary-source = Source: { $value }
canvas-summary-dir = Remote dir: { $value }
canvas-summary-domain = Domain: { $value }
canvas-summary-auto = Auto-deploy: { $value }
canvas-no-domain = (none)
canvas-logs-title = container logs
canvas-logs-offline = Connect the TARGET server, then reopen this tab to load logs.
canvas-logs-empty = No log lines yet. Select a container under App tools (t) or connect and refresh.
canvas-action-deploy = deploy
canvas-action-redeploy = redeploy
canvas-action-back = apps list
canvas-shortcut-pick = cycle account/repo/branch (↑↓)
canvas-shortcut-secrets = secrets

# Containers
containers-title = docker ps
containers-loading = loading…
containers-empty = no containers — connect under Servers (2) first
containers-shortcut-select = select
containers-shortcut-start = start
containers-shortcut-stop = stop
containers-shortcut-restart = restart
containers-shortcut-remove = remove
containers-shortcut-logs = logs
containers-shortcut-back = back

# Logs
logs-title = container logs
logs-target = container: { $name }
logs-fetching = fetching logs…
logs-empty = no logs yet
logs-shortcut-back = back

# Monitoring
monitoring-title = resource usage
monitoring-no-server = no server selected
monitoring-loading = Server: { $server }

    Loading metrics…
monitoring-not-connected = Server: { $server }

    Not connected — press [c] under Servers to connect.
monitoring-no-containers = Server: { $server }

    No running containers to measure.
    Deploy an app or start containers first.
monitoring-col-name = NAME
monitoring-col-cpu = CPU
monitoring-col-mem = MEM
monitoring-col-mem-pct = MEM%

# Schedules
schedules-title = restart policies & cron jobs
schedules-loading = Loading…
schedules-no-containers = No containers — connect to a server under Servers.
schedules-no-jobs =
    No cron jobs yet.
    Press a to schedule container restarts or compose redeploys.
    Example: 0 0 3 * * * = daily at 03:00 UTC.
schedules-shortcut-add = add cron
schedules-shortcut-toggle = toggle
schedules-shortcut-delete = delete
schedules-shortcut-select = select
schedules-action-restart = restart { $target }
schedules-action-redeploy = redeploy { $target }
schedules-last-never = never
schedules-form-restart = restart container
schedules-form-redeploy = redeploy compose dir
schedules-form-label = Label
schedules-form-cron = Cron
schedules-form-action = Action (Space toggles)
schedules-form-container = Container name
schedules-form-remote-dir = Remote dir
schedules-restart-policies = Docker restart policies
schedules-cron-panel = Cron jobs (runs while DokTUI is open)
schedules-cron-form-title = New cron job (Esc cancel, Enter save)
schedules-restart-line = { $name } — restart: { $policy } ({ $status })
schedules-status-on = on
schedules-status-off = off

# Secrets
secrets-title = encrypted local store
secrets-empty = no secrets yet — add one below (GitHub auth is under Git Providers)
secrets-key = Key:
secrets-value = Value:
secrets-shortcut-tab = field
secrets-shortcut-save = save
secrets-shortcut-delete = delete last
secrets-shortcut-back = back

# Provisioning
provision-title = installing Docker + Traefik
provision-panel-title = Provisioning Server
provision-starting = Starting…
provision-os = OS: { $os }

# Confirm dialog
confirm-title = Confirm
confirm-remove-container = Remove container '{ $name }'? This cannot be undone.
confirm-remove-server = Remove server '{ $name }'? This cannot be undone.
confirm-generic = Confirm action?
confirm-hint = Press [y] to confirm, [n] or Esc to cancel.

# Editor
editor-no-session = no editor session
editor-mode-vim = VIM
editor-mode-normal = NORMAL
editor-mode-insert = INSERT
editor-mode-edit = EDIT
editor-status-position = ln { $line }, col { $col }
editor-status-modified = +

# View panel titles
logs-panel-title = Logs
secrets-panel-title = Secrets / Env
containers-panel-title = Containers
deploy-panel-title = Deploy

# App status messages
status-copied-key = SSH public key copied
status-editor-saved = editor saved
status-provisioned = server provisioned successfully
status-cron-saved = cron job saved
status-cron-deleted = cron job deleted
status-server-removed = Server '{ $name }' removed
status-server-updated = server updated
status-connecting = connecting…
status-deploy-warnings = deploy completed with warnings
status-loading-github = loading GitHub repositories…
status-github-repos-loaded = GitHub repositories loaded
status-app-removed = app removed
status-auto-deploy = new commit detected — redeploying…
status-git-account-removed = GitHub account removed
status-git-connected = Connected GitHub account @{ $login }
achievement-first-https = First HTTPS Deploy

# Git Providers
git-panel-title = Git Providers
git-title = Git Providers
git-subtitle = Connect your Git provider for authentication
git-available = Available
git-connect-github = Connect GitHub
git-connected = Connected accounts
git-empty = No accounts yet. Press c to connect GitHub — your browser opens for login.
git-shortcut-connect = connect (browser)
git-shortcut-delete = delete account
git-shortcut-cancel-device = cancel
git-device-title = Connect GitHub
git-device-hint = Your browser should open. Enter this code at the URL below, then authorize DokTUI.
git-device-code = Your one-time code
git-device-starting = Starting GitHub Device Flow…
git-device-waiting = Waiting for authorization in browser…

# App errors
error-github-repo-required = GitHub owner and repo are required
error-clipboard = clipboard copy failed: { $err }
error-select-server = select a server under Servers (2) first
error-name-host-required = name and host are required
error-save-server = failed to save server
error-schedule-label = schedule label is required
error-secret-key = secret key is required
error-add-server-first = add a server first
error-hostkey-rejected = host key rejected — connection aborted
error-wildcard-dns =
    wildcard domains require DNS-01: set acme_challenge = "dns_cloudflare" in config and add CF_DNS_API_TOKEN in Secrets

# Command bus messages
cmd-no-ssh-session = no active SSH session — connect to a server first
cmd-no-ssh = no active SSH session
cmd-no-containers-logs = no containers to show logs for
cmd-saved-secret = saved secret `{ $key }`
cmd-removed-secret = removed secret `{ $key }`
cmd-server-not-found = server not found
cmd-connect-before-deploy = connect to server before deploying
cmd-traefik-not-running = Traefik is not running — provision the server first
cmd-cron-not-found = cron job not found
cmd-schedule-result = { $label }: { $msg }
cmd-container-action-ok = { $action } { $name } OK

# Provision progress
provision-detect-os = Detecting server OS…
provision-check-docker = Checking Docker…
provision-install-docker = Installing Docker (this may take a few minutes)…
provision-check-traefik = Checking Traefik…
provision-migrate-traefik = Migrating Traefik to doktui-network (auto-upgrade)…
provision-install-traefik = Deploying Traefik…
provision-verify = Verifying setup…
provision-ready = Server ready

# Routing validation
error-domain-empty = domain cannot be empty when routing is enabled
error-wildcard-format = wildcard domain must start with *., e.g. *.example.com
error-compose-service = compose service name is required
error-invalid-port = port must be between 1 and 65535
error-invalid-host = invalid hostname

# Theme card
card-getting-started = getting started

# Block titles
block-navigation = navigation
