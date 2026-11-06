# Moth Controller — anonymous active installations

This is **installation telemetry**, not PokéIdle-player analytics. One desktop installation is counted once, regardless of how many accounts it runs. A user with two computers may be counted twice. This feature begins collecting **only after** you deploy the receiver and release a binary that includes its URL; it cannot recover historical active-user counts.

## Deploy (Cloudflare Workers + D1)

1. Install Wrangler and sign in: `npx wrangler login`.
2. In `telemetry/`, create the database: `npx wrangler d1 create moth-active-users`.
3. Copy `wrangler.toml.example` to `wrangler.toml`. Paste the database UUID printed in step 2. Do **not** commit the configured file; keep it local.
4. Apply the schema: `npx wrangler d1 execute moth-active-users --remote --file=schema.sql`.
5. Set a long random admin secret with `npx wrangler secret put ADMIN_TOKEN`.
6. Deploy from `telemetry/` using `npx wrangler deploy`. This gives you a URL such as `https://moth-active-users.<your-subdomain>.workers.dev`.
7. In the **pokeidle-addons** GitHub repository, open Settings → Secrets and variables → Actions → **Variables**. Add `MOTH_TELEMETRY_URL` with the full **HTTPS** URL including `/v1/heartbeat`. This is a public write endpoint, not the admin secret.
8. Publish a **new Moth release** after setting the variable. The Windows and Linux release builds embed that URL. Existing builds will not report automatically.

The read API is intentionally private:

```bash
curl -H "Authorization: Bearer YOUR_ADMIN_TOKEN" https://moth-active-users.YOUR-SUBDOMAIN.workers.dev/v1/stats
```

Example response:

```json
{
  "generated_at": "2026-11-06T12:00:00.000Z",
  "totals": { "active_24h": 14, "active_7d": 19, "active_30d": 26 },
  "by_version_platform": [
    { "version": "0.2.7", "platform": "windows", "active_24h": 12, "active_7d": 16, "active_30d": 20 }
  ]
}
```

The figures above are **illustrative**, not actual telemetry.

## Client privacy and behavior

- Settings → Updates → **Send anonymous daily usage statistics** lets users opt out. Default is enabled. Without a configured HTTPS endpoint, no worker/network request runs.
- One locally generated random 32-character installation identifier, app version and operating system. No usernames, account IDs, IP addresses in the database, tokens, gameplay, stream identities, diagnostic data or local machine fingerprint.
- The client reports at most once every 24 hours after a successful report. Failed reports retry no sooner than one hour later. Sends are asynchronous and never block the UI or gameplay.
- Clearing the app data directory resets the anonymous ID and can make the same physical device appear as a new installation.
- The receiver keeps only last-seen time and latest version per installation. Querying statistics removes rows inactive for more than 30 days. No history of individual sessions is collected.
- No user-facing login is required. Protect `ADMIN_TOKEN` and avoid turning on request-body logging. Cloudflare may process transient connection IPs under its standard infrastructure operations, but the worker does not store them.
- Treat counts as **active installations that opted in and reached the receiver**, not exact human-user counts. The service must be up, and the binary must have a configured receiver URL.

## Operational safety

The receiver accepts public POSTs, so reports can be forged. The metrics are adequate for rough adoption estimates, not security-sensitive billing, licensing or fraud decisions. The read API requires `ADMIN_TOKEN`. Use Cloudflare rate limiting if the endpoint attracts abuse. Watch D1/Workers free-tier limits for unusually high traffic.

The worker reads no GitHub credentials; the release build needs only a public endpoint URL. Do not place the admin token in GitHub Actions variables or in the controller.
