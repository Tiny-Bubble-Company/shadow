// Worker for https://getshadow.dev: serves the built site (dist/, via the ASSETS binding),
// sends www.getshadow.dev to the apex domain with a 301 (path and query kept), redirects
// moved pages, and answers two small endpoints:
//
//   POST /api/e     first-party funnel events -> Analytics Engine (see worker/README.md)
//   POST /api/form  waitlist and demo forms, verified with Turnstile, stored in KV
//
// No cookies are set anywhere, no IP address or full user agent is stored, and only the
// coarse country from Cloudflare's header is kept with an event.
const APEX = 'getshadow.dev';

// Moved pages: old path -> new path (both with trailing slash).
const REDIRECTS = {
	'/compare/ai-dlp-tools/': '/compare/ai-dlp/',
};

// Allowed events and the props each may carry (anything else is dropped).
const EVENTS = {
	install_copy: ['method'],
	hub_start_free: ['location'],
	book_demo: ['location'],
	waitlist_submit: ['location'],
	hub_connect_page: ['source'],
	pricing_tier_click: ['tier', 'billing'],
	compare_cta: ['competitor'],
};
const FORM_TYPES = ['waitlist', 'demo'];

// Requests per minute per event type, per Cloudflare location (isolates are per location).
const RATE_PER_MINUTE = 600;
const counters = new Map();
function limited(key) {
	const minute = Math.floor(Date.now() / 60000);
	const k = `${key}:${minute}`;
	const n = (counters.get(k) || 0) + 1;
	counters.set(k, n);
	if (counters.size > 1000) {
		for (const old of counters.keys()) if (!old.endsWith(`:${minute}`)) counters.delete(old);
	}
	return n > RATE_PER_MINUTE;
}

const NO_STORE = { 'cache-control': 'no-store', 'x-content-type-options': 'nosniff' };
const empty = (status) => new Response(null, { status, headers: NO_STORE });
const json = (status, body) =>
	new Response(JSON.stringify(body), { status, headers: { ...NO_STORE, 'content-type': 'application/json' } });

function clean(value, max = 48) {
	return String(value ?? '')
		.replace(/[^\w .:/@+-]/g, '')
		.slice(0, max);
}

/** Path only: never a query string or fragment (a workspace token must not be logged). */
function cleanPath(p) {
	try {
		return new URL(String(p || '/'), `https://${APEX}`).pathname.slice(0, 200);
	} catch {
		return '/';
	}
}

async function readJson(request, maxBytes) {
	const text = await request.text();
	if (text.length > maxBytes) return null;
	try {
		return JSON.parse(text);
	} catch {
		return null;
	}
}

async function handleEvent(request, env) {
	const body = await readJson(request, 2048);
	if (!body || !Object.hasOwn(EVENTS, body.event)) return empty(204); // unknown: ignore
	if (limited(body.event)) return empty(204);
	const props = {};
	for (const k of EVENTS[body.event]) {
		if (body.props && body.props[k] != null) props[k] = clean(body.props[k]);
	}
	const country = clean(request.headers.get('cf-ipcountry') || 'XX', 2);
	if (env.EVENTS) {
		env.EVENTS.writeDataPoint({
			indexes: [body.event],
			blobs: [body.event, cleanPath(body.path), JSON.stringify(props), country],
			doubles: [1],
		});
	}
	return empty(204);
}

async function verifyTurnstile(token, env) {
	if (!env.TURNSTILE_SECRET || !token) return false;
	const form = new FormData();
	form.append('secret', env.TURNSTILE_SECRET);
	form.append('response', String(token));
	const r = await fetch('https://challenges.cloudflare.com/turnstile/v0/siteverify', { method: 'POST', body: form });
	const out = await r.json().catch(() => ({}));
	return out.success === true;
}

async function handleForm(request, env) {
	if (!env.FORMS || !env.TURNSTILE_SECRET) {
		return json(503, { error: 'forms are not configured yet', fallback: 'mailto:support@getshadow.dev' });
	}
	if (limited('form')) return json(429, { error: 'too many requests, try again in a minute' });
	const body = await readJson(request, 8192);
	if (!body || !FORM_TYPES.includes(body.form_type)) return json(400, { error: 'unknown form' });
	const email = String(body.email || '').trim().toLowerCase();
	if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email) || email.length > 200) {
		return json(400, { error: 'enter a valid work email' });
	}
	if (!(await verifyTurnstile(body.turnstile, env))) return json(403, { error: 'verification failed, please retry' });
	const record = {
		form_type: body.form_type,
		email,
		name: clean(body.name, 120),
		company: clean(body.company, 120),
		team_size: clean(body.team_size, 16),
		message: String(body.message || '').slice(0, 2000),
		location: clean(body.location, 24),
		tags: body.form_type === 'waitlist' ? ['hub-waitlist'] : ['hub-demo'],
		country: clean(request.headers.get('cf-ipcountry') || 'XX', 2),
		at: new Date().toISOString(),
	};
	await env.FORMS.put(`${body.form_type}:${record.at}:${crypto.randomUUID()}`, JSON.stringify(record));
	return json(200, { ok: true });
}

export default {
	async fetch(request, env) {
		const url = new URL(request.url);
		if (url.hostname === `www.${APEX}`) {
			url.hostname = APEX;
			url.protocol = 'https:';
			return Response.redirect(url.toString(), 301);
		}
		if (url.pathname === '/api/e') {
			return request.method === 'POST' ? handleEvent(request, env) : empty(405);
		}
		if (url.pathname === '/api/form') {
			return request.method === 'POST' ? handleForm(request, env) : empty(405);
		}
		const path = url.pathname.endsWith('/') ? url.pathname : `${url.pathname}/`;
		if (Object.hasOwn(REDIRECTS, path)) {
			url.pathname = REDIRECTS[path];
			return Response.redirect(url.toString(), 301);
		}
		return env.ASSETS.fetch(request);
	},
};
