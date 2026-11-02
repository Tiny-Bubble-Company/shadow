# getshadow.dev Worker

`index.js` serves the built site and adds:

- **www → apex**: `https://www.getshadow.dev/*` returns a 301 to `https://getshadow.dev/*` with the path and query kept.
- **Moved pages**: the `REDIRECTS` map returns a 301 from old paths to new ones, so indexed URLs never 404.
- **`POST /api/e`**: first-party funnel events.
- **`POST /api/form`**: the Hub waitlist and Book a demo forms.

No response sets a cookie.

## Funnel events (`/api/e`)

The site sends `{event, path, props}` with `navigator.sendBeacon` (see `src/scripts/track.ts`). The Worker:

- ignores unknown events and unknown props;
- keeps only the path of `path` (never the query string, so a workspace token in a URL is never logged);
- keeps the coarse country from `CF-IPCountry`, and never the IP address or user agent;
- rate-limits each event type to 600 per minute per Cloudflare location;
- writes one Analytics Engine data point: `indexes = [event]`, `blobs = [event, path, props JSON, country]`, `doubles = [1]`.

| Event | Fired on | Props |
|---|---|---|
| `install_copy` | an install command is copied | `method`: `script`, `powershell`, `cargo`, `docker` |
| `hub_start_free` | Start free Hub trial (or Join the Hub waitlist while Hub is not live) | `location`: `hero`, `header`, `pricing`, `hub`, `compare`, `footer` |
| `book_demo` | Book a demo form submitted, or the booking link opened | `location` |
| `waitlist_submit` | waitlist form submitted successfully | `location` |
| `hub_connect_page` | `/hub/connect/` viewed | `source`: `cli`, `invite`, `other` |
| `pricing_tier_click` | a tier button on `/pricing/` | `tier`, `billing`: `annual`, `monthly` |
| `compare_cta` | a CTA on a `/compare/*` page | `competitor` |

Query the data with the Analytics Engine SQL API, for example:

```sql
SELECT blob1 AS event, blob2 AS path, count() AS n
FROM shadow_funnel
WHERE timestamp > NOW() - INTERVAL '7' DAY
GROUP BY event, path ORDER BY n DESC
```

## Forms (`/api/form`)

Body: `{form_type: "waitlist" | "demo", email, name?, company?, team_size?, message?, location?, turnstile}`.
The Worker verifies the Turnstile token, then stores the submission in the `FORMS` KV namespace under `<form_type>:<ISO time>:<uuid>`. Waitlist entries carry the tag `hub-waitlist`.

Until both are configured, `/api/form` answers `503` and the forms show a "write to support@getshadow.dev" fallback instead.

To turn the forms on:

1. Create a Turnstile widget for `getshadow.dev` in the Cloudflare dashboard.
2. Build with the site key: `PUBLIC_TURNSTILE_SITE_KEY=<site key> pnpm build` (set it as a repository variable for the deploy workflow).
3. Store the secret: `npx wrangler secret put TURNSTILE_SECRET`.
4. Create the KV namespace (`npx wrangler kv namespace create FORMS`) and add it to `wrangler.jsonc` under `kv_namespaces` with binding `FORMS`.
