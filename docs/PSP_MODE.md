# PSP Mode — Podman Socket Proxy Integration

PSP mode gives sandbox agents access to container operations (pull images, run containers, inspect state) through a policy-gated proxy instead of a raw Podman socket.

---

## Quick start

```bash
# Install PSP (see https://github.com/user/podman-socket-proxy)
cargo install --path ../podman-socket-proxy

# Run with PSP enabled
ags --agent pi --psp
```

## How it works

1. AGS spawns a `psp` sidecar process with a per-run Unix socket.
2. AGS waits for PSP to become ready (socket accepts connections).
3. The socket directory is mounted into the container at `/run/psp/`.
4. `DOCKER_HOST=unix:///run/psp/psp.sock` is injected into the container environment.
5. Docker/Testcontainers clients inside the sandbox route through PSP.
6. When the container exits, AGS sends SIGTERM to PSP for graceful cleanup, then SIGKILL after 5 seconds.

## Flags

| Flag | Effect |
|------|--------|
| `--psp` | Enable PSP mode (starts sidecar, mounts socket, sets DOCKER_HOST) |
| `--psp-keep` | Keep PSP-managed containers on exit for debugging |

## Config

The only config option is an optional binary path override:

```toml
# ~/.config/ags/config.toml
[psp]
binary = "/usr/local/bin/psp"   # default: looks up `psp` on PATH
```

Enabling PSP is **only** via `--psp` flag, not config. This is intentional — PSP mode changes the security surface and should be an explicit opt-in.

## Environment variables

| Variable | Set by | Where | Purpose |
|----------|--------|-------|---------|
| `DOCKER_HOST` | AGS | Container | Points Docker/Testcontainers to PSP socket |
| `PSP_SESSION_ID` | AGS | Container | Stable session identifier (`ags-{agent}-{pid}`) for `x-psp-session-id` header |
| `TESTCONTAINERS_HOST_OVERRIDE` | AGS | Container | Routes Testcontainers connections to `host.containers.internal` |
| `PSP_LISTEN_SOCKET` | AGS | PSP process | Tells PSP where to create its socket |
| `PSP_KEEP_ON_FAILURE` | AGS (via `--psp-keep`) | PSP process | Retains containers for debugging |

## Security boundaries

### AGS is responsible for:

- Starting/stopping the PSP sidecar process
- Socket isolation (per-PID socket directory)
- Mounting the socket into the container
- Setting `DOCKER_HOST` so tools use PSP instead of a raw socket
- Graceful cleanup (SIGTERM → wait → SIGKILL)

### PSP is responsible for:

- Policy enforcement (deny-by-default, image allowlists, bind mount restrictions)
- Container labeling and tracking (`io.psp.managed=true`, `io.psp.session={id}`)
- Startup sweep (removing stale managed containers from previous crashes)
- Graceful container cleanup on exit
- Host IP rewriting in container inspect responses
- Request/session correlation headers

### Neither AGS nor PSP:

- Manages PSP policy files — users create and maintain their own policies
- Grants raw Podman socket access — the raw socket is never mounted

## PSP policy files

PSP loads policies from (in order, later overrides):

1. Built-in defaults (deny all)
2. `~/.config/psp/config.json` (global)
3. `<repo-root>/.psp.json` (project-local, checked into version control)

Example project-local policy (`.psp.json`):

```json
{
  "allowed_images": ["docker.io/library/postgres:*", "docker.io/library/redis:*"],
  "allow_bind_mounts": false
}
```

See PSP documentation for full policy reference.

## Troubleshooting

### `psp binary not found`

Install PSP and ensure it's on PATH, or set `[psp].binary` in config:

```bash
which psp            # verify it's on PATH
psp --version        # verify it runs
```

### `psp: timed out waiting for readiness (10s)`

PSP didn't become ready within the timeout. Check:

- Is Podman socket available? `ls $XDG_RUNTIME_DIR/podman/podman.sock`
- Is the policy file valid? PSP logs to stderr (visible in AGS output).

### `psp: process exited immediately`

PSP started but crashed. Common causes:

- Invalid policy file (bad JSON)
- Missing Podman socket
- Port/socket conflict

Check stderr output for PSP error messages.

### Containers not cleaned up

If PSP containers persist after exit:

- Without `--psp-keep`: PSP should clean up on next start (startup sweep). Run `ags --agent shell --psp` to trigger cleanup.
- With `--psp-keep`: This is expected. Remove manually: `podman rm -f $(podman ps -aq --filter label=io.psp.managed=true)`

### Testcontainers can't connect

Verify inside the container:

```bash
echo $DOCKER_HOST                    # should be unix:///run/psp/psp.sock
curl --unix-socket /run/psp/psp.sock http://localhost/_ping   # should return OK
```

## Migration from raw socket mounts

If you were previously mounting the Podman socket directly:

```toml
# OLD — remove this
[[mount]]
host = "~/.local/share/containers/podman.sock"
container = "/var/run/docker.sock"
```

Replace with:

```bash
# NEW — use PSP mode
ags --agent pi --psp
```

Benefits:

- **Policy enforcement**: deny-by-default vs. full root-equivalent access
- **Container tracking**: PSP tracks what it creates, cleans up on exit
- **Audit trail**: `x-psp-request-id` headers for log correlation
- **No privilege escalation**: PSP proxies only supported endpoints (8-10 operations)
