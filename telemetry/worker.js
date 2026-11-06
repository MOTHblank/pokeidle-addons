// Cloudflare Worker + D1: receives one anonymous heartbeat per installation/day.
// No cookies, IP-address storage, account names, game telemetry, or request logs.
// Set ADMIN_TOKEN via wrangler secret put ADMIN_TOKEN.
const json = (value, status = 200) => new Response(JSON.stringify(value), {
  status,
  headers: { "content-type": "application/json; charset=utf-8", "cache-control": "no-store" },
});
const validVersion = (value) => typeof value === "string" && /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(value) && value.length <= 48;
const validId = (value) => typeof value === "string" && /^[0-9a-f]{32}$/.test(value);

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.pathname === "/v1/heartbeat" && request.method === "POST") {
      if (Number(request.headers.get("content-length") || 0) > 1024) return json({ error: "too large" }, 413);
      let body;
      try { body = await request.json(); } catch { return json({ error: "invalid JSON" }, 400); }
      if (!body || !validId(body.installation_id) || !validVersion(body.version) ||
          !["windows", "linux", "other"].includes(body.platform)) {
        return json({ error: "invalid heartbeat" }, 400);
      }
      const now = Math.floor(Date.now() / 1000);
      await env.DB.prepare(
        `INSERT INTO installations (id, version, platform, last_seen)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET
           version=excluded.version, platform=excluded.platform,
           last_seen=excluded.last_seen`
      ).bind(body.installation_id, body.version, body.platform, now).run();
      return new Response(null, { status: 204 });
    }

    if (url.pathname === "/v1/stats" && request.method === "GET") {
      const secret = env.ADMIN_TOKEN;
      if (!secret || request.headers.get("authorization") !== `Bearer ${secret}`) {
        return json({ error: "unauthorized" }, 401);
      }
      const now = Math.floor(Date.now() / 1000);
      const window = now - 30 * 86400;
      await env.DB.prepare("DELETE FROM installations WHERE last_seen < ?1").bind(window).run();
      const { results } = await env.DB.prepare(
        `SELECT version, platform,
         SUM(CASE WHEN last_seen >= ?1 THEN 1 ELSE 0 END) AS active_24h,
         SUM(CASE WHEN last_seen >= ?2 THEN 1 ELSE 0 END) AS active_7d,
         COUNT(*) AS active_30d
         FROM installations GROUP BY version, platform ORDER BY active_7d DESC, version DESC`
      ).bind(now - 86400, now - 7 * 86400).all();
      const totals = results.reduce((acc, item) => {
        for (const field of ["active_24h", "active_7d", "active_30d"]) acc[field] += item[field];
        return acc;
      }, { active_24h: 0, active_7d: 0, active_30d: 0 });
      return json({ generated_at: new Date().toISOString(), totals, by_version_platform: results });
    }

    return json({ error: "not found" }, 404);
  },
};
