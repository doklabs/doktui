# Brand & chrome
brand-name = DokTUI
brand-subtitle = doklabs
brand-tagline = local TUI for remote deployments
brand-tagline-short = DokTUI · local TUI

# Navigation
nav-home = Home
nav-projects = Projects
nav-deployments = Deployments
nav-monitoring = Monitoring
nav-schedules = Schedules
nav-navigation = NAVIGATION
nav-servers = SERVERS
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

    Use Projects to manage SSH servers.
    Use Deployments to deploy apps.
home-no-servers = No servers yet.

    Go to Projects → press [a] to register an SSH server.
home-servers-registered =
    { $count } server(s) registered.
    Select one under Projects.
home-achievement-label = ACHIEVEMENT
home-achievement-https = Let's Encrypt cert issued · +50 XP
home-deploy-panel = Quick deploy
home-deploy-placeholder = app.example.com
home-deploying-title = { $arrow } DEPLOYING · { $domain }
home-deploy-building = building { $spin }
home-deploy-network-attached = { $check } doktui-network attached
home-deploy-traefik-route = { $check } traefik route → Host(`{ $domain }`) tls:le
home-deploy-pulling = { $arrow } pulling image…

# Projects / servers
servers-title = SSH targets
servers-shortcut-select = select
servers-shortcut-connect = connect
servers-shortcut-provision = provision
servers-shortcut-add = add
servers-shortcut-open = open
servers-shortcut-remove = remove
servers-shortcut-back = back

# Deployments hub
deploy-hub-title = deploy & runtime
deploy-hub-target = Target: { $name }
deploy-hub-no-target = Target: (none — pick server in Projects)
deploy-hub-item-deploy = [d] Deploy — docker compose to server
deploy-hub-item-containers = [c] Containers — start/stop/restart
deploy-hub-item-logs = [l] Logs — stream container output
deploy-hub-item-secrets = [s] Secrets — env vars (encrypted locally)
deploy-hub-item-editor = [e] Editor — edit compose file
deploy-hub-shortcut-deploy = deploy
deploy-hub-shortcut-containers = containers
deploy-hub-shortcut-logs = logs
deploy-hub-shortcut-secrets = secrets
deploy-hub-shortcut-editor = editor
deploy-hub-shortcut-nav = navigate
deploy-hub-shortcut-open = open

# Deploy form
deploy-title = docker compose + traefik routing
deploy-field-remote-dir = Remote dir
deploy-field-domain = Domain (or *.example.com)
deploy-field-port = Port
deploy-field-service = Service
deploy-field-https = HTTPS (Let's Encrypt)
deploy-field-compose = Compose
deploy-on = on
deploy-off = off
deploy-compose-editing = (editing — press e for canvas editor)
deploy-compose-lines = { $count } lines
deploy-shortcut-tab = field
deploy-shortcut-https = toggle HTTPS
deploy-shortcut-deploy = deploy
deploy-shortcut-editor = editor

# Containers
containers-title = docker ps
containers-loading = loading…
containers-empty = no containers — connect to a server under Projects first
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

    Not connected — press [c] under Projects to connect.
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
schedules-no-containers = No containers — connect to a server under Projects.
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
secrets-empty = no secrets yet — add one below
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
status-connecting = connecting…
status-deploy-warnings = deploy completed with warnings
achievement-first-https = First HTTPS Deploy

# App errors
error-clipboard = clipboard copy failed: { $err }
error-select-server = select a server under Projects first
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
