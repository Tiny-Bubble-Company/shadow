# Production image for the Shadow website: build the static site with pnpm, serve it with Caddy.
#
#   docker build -f deploy/Dockerfile --build-arg SITE=https://example.com -t shadow-site .
#
# Run from docs-site/ (the build context). See deploy/README.md.

FROM node:22-alpine AS build
WORKDIR /site
RUN corepack enable
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY . .
# Public origin of the site (required): canonical URLs, sitemap, robots.txt, llms.txt, JSON-LD.
ARG SITE
# Path prefix; leave empty to serve at the domain root.
ARG BASE=""
RUN if [ -z "$SITE" ]; then echo "error: pass --build-arg SITE=https://<your-domain>" >&2; exit 1; fi
RUN SITE="$SITE" BASE="$BASE" pnpm build \
	&& node deploy/csp.mjs dist /site/csp.caddy

FROM caddy:2-alpine
COPY deploy/Caddyfile /etc/caddy/Caddyfile
COPY --from=build /site/csp.caddy /etc/caddy/csp.caddy
COPY --from=build /site/dist /srv
EXPOSE 80 443
