# Self-hosting the Shadow website (Hetzner or any VM)

The site is static. The image in this folder builds it with pnpm and serves `dist/` with
[Caddy](https://caddyserver.com/), which gets and renews HTTPS certificates on its own.

What you get: automatic HTTPS, `www.` → apex redirect, zstd/gzip compression, one-year
immutable caching for fingerprinted assets (`/_astro/*`), revalidation for pages, security
headers (HSTS, nosniff, frame denial, referrer and permissions policies) and a
Content-Security-Policy that allows the site's inline scripts by hash (generated at build
time by `csp.mjs`), and the site's own 404 page.

## 1. Create the server

1. In the Hetzner Cloud console create a server: Ubuntu 24.04, the smallest shared-vCPU type
   is plenty (for example CX22 or CAX11), add your SSH key.
2. Firewall: allow inbound TCP 22, 80, 443 and UDP 443 (HTTP/3).
3. Install Docker:

   ```sh
   ssh root@<server-ip>
   curl -fsSL https://get.docker.com | sh
   ```

## 2. Point DNS at it

At your domain registrar (or DNS provider) create:

| Type | Name | Value |
|---|---|---|
| A | `@` | `<server IPv4>` |
| AAAA | `@` | `<server IPv6>` (optional) |
| CNAME | `www` | `<your domain>` |

Wait until `dig +short <your domain>` returns the server IP. Caddy can only get a certificate
once DNS resolves to the server and ports 80/443 are open.

## 3. Build and run

```sh
git clone https://github.com/Tiny-Bubble-Company/shadow.git
cd shadow/docs-site/deploy
cp .env.example .env          # set DOMAIN=<your domain>, no https://
docker compose up -d --build
docker compose logs -f        # watch Caddy obtain the certificate
```

The compose file passes `SITE=https://$DOMAIN` to the build, so canonical URLs, the sitemap,
`robots.txt`, `llms.txt` and JSON-LD all use your domain. The site is served from the domain
root (`BASE` empty).

To deploy a new version: `git pull && docker compose up -d --build`.

Certificates live in the `caddy_data` volume; keep it across rebuilds to avoid hitting
Let's Encrypt rate limits.

## Building without Docker

```sh
cd docs-site
pnpm install
SITE=https://<your domain> pnpm build      # BASE defaults to '' (domain root)
node deploy/csp.mjs dist deploy/csp.caddy  # only needed if you serve with this Caddyfile
```

Then serve `dist/` with any static web server (and copy `csp.caddy` to `/etc/caddy/` if you
use `Caddyfile` directly).

## Notes

- `SITE` is required for production builds. Without it the build warns and falls back to
  `http://localhost:4321`.
- `BASE=/shadow` builds for a sub-path (GitHub Pages project site); leave it unset here.
- Page dates in structured data come from git history. The Docker build context has no
  `.git`, so pages get the build time as `dateModified`; build outside Docker (or copy `.git`
  in) if you want per-page dates.
- If you add an inline `<script>` to the site, the CSP picks it up on the next build because
  `csp.mjs` hashes every inline script in `dist/`.
