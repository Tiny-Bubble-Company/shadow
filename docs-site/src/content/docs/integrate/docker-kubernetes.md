---
title: Docker and Kubernetes
description: "Run Shadow as a sidecar container in Docker or Kubernetes to mask PII in your service's LLM calls, with persistent state and controlled egress."
---

In containers, Shadow works best as a **sidecar**: one Shadow instance next to each application, reachable on `localhost`, using [gateway mode](/shadow/integrate/gateway-mode/). No CA has to be installed in the application image.

## Build the image

```sh
docker build -f packaging/Dockerfile -t shadow:local .
```

The image is based on `debian:bookworm-slim`, runs as the unprivileged user `shadow` (UID 10001), stores its state in `/data` (`SHADOW_HOME=/data`), exposes port 8877, and has a `HEALTHCHECK` that runs `shadow status --quiet`. The default command is `shadow run --listen 0.0.0.0:8877`.

Configuration goes in `/data/shadow.toml`. Mount it from a config map or a file, or let Shadow use its defaults.

## Docker Compose

```yaml
services:
  app:
    image: my-app:latest
    environment:
      OPENAI_BASE_URL: http://shadow:8877/gw/openai/v1
      ANTHROPIC_BASE_URL: http://shadow:8877/gw/anthropic
    depends_on: [shadow]

  shadow:
    image: shadow:local
    volumes:
      - shadow-data:/data
      - ./shadow.toml:/data/shadow.toml:ro
    # No published ports: only containers on this network can reach it.

volumes:
  shadow-data:
```

## Kubernetes sidecar

Containers in one pod share a network namespace, so the app reaches Shadow on `127.0.0.1` and Shadow can listen on loopback only:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-app
spec:
  replicas: 1
  selector: { matchLabels: { app: my-app } }
  template:
    metadata: { labels: { app: my-app } }
    spec:
      containers:
        - name: app
          image: my-app:latest
          env:
            - name: OPENAI_BASE_URL
              value: http://127.0.0.1:8877/gw/openai/v1
        - name: shadow
          image: registry.example.com/shadow:0.1.0
          args: ["run", "--listen", "127.0.0.1:8877"]
          volumeMounts:
            - { name: shadow-data, mountPath: /data }
            - { name: shadow-config, mountPath: /data/shadow.toml, subPath: shadow.toml }
          readinessProbe:
            exec: { command: ["shadow", "status", "--quiet"] }
          securityContext:
            runAsNonRoot: true
            allowPrivilegeEscalation: false
            readOnlyRootFilesystem: true
      volumes:
        - name: shadow-data
          persistentVolumeClaim: { claimName: my-app-shadow }
        - name: shadow-config
          configMap: { name: my-app-shadow }
```

### State and replicas

The data directory holds the persona, the vault and its key. It matters in two ways:

- **Restoring responses** needs the mapping that masked the request. Within one request this is always in memory, so a lost volume never breaks an in-flight call.
- **Stable fakes over time** (the same customer always appears under the same fake name, conversation history stays consistent) need the vault to persist.

For one replica, a `PersistentVolumeClaim` is enough. For several replicas that should present one identity, either give them a shared `ReadWriteMany` volume (Shadow merges concurrent writers on save), or provision the same `persona.json`, `vault.key` and an initial `vault.bin` from a secret into each pod. Treat `vault.key` as a secret.

## Proxy mode in containers

If the application cannot set a base URL, use proxy mode inside the pod: set `HTTPS_PROXY=http://127.0.0.1:8877` and make the application trust Shadow's CA. Fetch it from the running sidecar at `http://127.0.0.1:8877/shadow/ca.pem`, or mount `ca/ca.pem` from the shared volume. Provision the CA ahead of time (run `shadow init` once and store `ca/` as a secret) so that it is stable across restarts.

## Egress control

The sidecar is the only component that needs to reach AI providers. A `NetworkPolicy` applies to the whole pod, so it cannot stop the application container from bypassing a sidecar in the same pod. To enforce that, route egress through a proxy or firewall that only accepts provider traffic from Shadow (point Shadow at it with `upstream_proxy`), or run Shadow as a separate gateway deployment and allow provider egress only from its pods. Set `[proxy] upstream_proxy` too if outbound traffic must go through a corporate proxy or a relay.

## Without containers

On a VM, the same pattern is a systemd unit (see [Installation](/shadow/getting-started/installation/#run-as-a-background-service)) and gateway URLs in the application's environment.
