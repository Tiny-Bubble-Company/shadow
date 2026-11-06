// Worker for https://getshadow.dev: serves the built site (dist/, via the ASSETS binding),
// sends www.getshadow.dev to the apex domain with a 301 (path and query kept), redirects
// moved pages, and answers two small endpoints:
//
//   POST /api/e     first-party funnel events -> Analytics Engine (see worker/README.md)
//   POST /api/form  waitlist and demo forms, verified with Turnstile, stored in KV
//   POST /api/support  help and feedback from Shadow Hub, emailed to the support inbox
//
// No cookies are set anywhere, no IP address or full user agent is stored, and only the
// coarse country from Cloudflare's header is kept with an event.
import { EmailMessage } from 'cloudflare:email';

const APEX = 'getshadow.dev';

// Moved pages: old path -> new path (both with trailing slash).
// Guide URLs redirect to /research/ for SEO preservation (marketing guides moved to editorial layout).
const REDIRECTS = {
	'/compare/ai-dlp-tools/': '/compare/ai-dlp/',
	// Marketing guides migrated to research section
	'/guides/does-cursor-train-on-my-code/': '/research/does-cursor-train-on-my-code/',
	'/guides/private-chatgpt/': '/research/private-chatgpt/',
	'/guides/is-chatgpt-safe-for-company-data/': '/research/is-chatgpt-safe-for-company-data/',
	'/guides/what-is-an-llm-proxy/': '/research/what-is-an-llm-proxy/',
	'/guides/what-is-shadow-ai/': '/research/what-is-shadow-ai/',
	'/guides/ai-acceptable-use-policy-template/': '/research/ai-acceptable-use-policy-template/',
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
const SUPPORT_KINDS = { bug: 'Bug', feature: 'Feature request', question: 'Question', billing: 'Billing' };
const SUPPORT_FROM = `hub@${APEX}`;

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

/** Header-safe text: one line, no control characters. */
const oneLine = (v, max) =>
	String(v ?? '')
		.replace(/[\u0000-\u001f\u007f]+/g, ' ')
		.trim()
		.slice(0, max);

const b64 = (text) => {
	const bytes = new TextEncoder().encode(text);
	let bin = '';
	for (const b of bytes) bin += String.fromCharCode(b);
	return btoa(bin);
};

/** RFC 2047 encoded header value for non-ASCII text. */
const encodeWord = (text) => (/^[\x20-\x7e]*$/.test(text) ? text : `=?UTF-8?B?${b64(text)}?=`);

async function sameToken(a, b) {
	const enc = new TextEncoder();
	const [x, y] = [enc.encode(a), enc.encode(b)];
	return x.byteLength === y.byteLength && crypto.subtle.timingSafeEqual(x, y);
}

// Help and feedback sent from inside Shadow Hub. Only the Hub server may call this (shared
// SUPPORT_RELAY_TOKEN); the message is emailed to SUPPORT_INBOX through Email Routing with the
// customer as Reply-To, so the team answers by replying.
async function handleSupport(request, env) {
	if (!env.SUPPORT_MAIL || !env.SUPPORT_INBOX || !env.SUPPORT_RELAY_TOKEN) {
		return json(503, { error: 'support relay is not configured' });
	}
	const auth = request.headers.get('authorization') || '';
	if (!auth.startsWith('Bearer ') || !(await sameToken(auth.slice(7), env.SUPPORT_RELAY_TOKEN))) {
		return json(401, { error: 'unauthorized' });
	}
	if (limited('support')) return json(429, { error: 'too many requests' });
	const body = await readJson(request, 32768);
	if (!body || !Object.hasOwn(SUPPORT_KINDS, body.kind)) return json(400, { error: 'bad message' });
	const email = oneLine(body.email, 200).toLowerCase();
	if (!/^[^\s@<>",]+@[^\s@<>",]+\.[^\s@<>",]+$/.test(email)) return json(400, { error: 'bad sender' });
	const name = oneLine(body.name, 120).replace(/["\\]/g, '');
	const subject = oneLine(body.subject, 200);
	const message = String(body.message ?? '').slice(0, 5000);
	const ctx = body.context && typeof body.context === 'object' ? body.context : {};
	const workspace = oneLine(ctx.workspace, 120);
	const kind = SUPPORT_KINDS[body.kind];
	const created = Number(ctx.workspace_created_at);
	const lines = [
		message,
		'',
		'-- ',
		`${kind} from Shadow Hub`,
		`From:        ${name} <${email}> (${oneLine(ctx.role, 20)})`,
		`Workspace:   ${workspace} (${oneLine(ctx.workspace_id, 64)})`,
		`Plan:        ${oneLine(ctx.plan, 80)}`,
		`People:      ${Number(ctx.people) || 0} connected, ${Number(ctx.devices) || 0} devices`,
		`Core:        ${oneLine(ctx.core_versions, 200) || 'no devices yet'}`,
		`Hub:         ${oneLine(ctx.hub_url, 120)} (version ${oneLine(ctx.hub_version, 20)})`,
		`Created:     ${created ? new Date(created).toISOString().slice(0, 10) : 'unknown'}`,
		`Browser:     ${oneLine(ctx.browser, 300)}`,
		`Message ID:  ${oneLine(body.id, 64)}`,
		'',
		'Reply to this email to answer the customer.',
	];
	const raw = [
		`From: ${encodeWord('Shadow Hub')} <${SUPPORT_FROM}>`,
		`To: <${env.SUPPORT_INBOX}>`,
		`Reply-To: ${name ? (encodeWord(name) === name ? `"${name}" ` : `${encodeWord(name)} `) : ''}<${email}>`,
		`Subject: ${encodeWord(`[${kind}] ${subject}${workspace ? ` (${workspace})` : ''}`)}`,
		`Message-ID: <${crypto.randomUUID()}@${APEX}>`,
		`Date: ${new Date().toUTCString()}`,
		'MIME-Version: 1.0',
		'Content-Type: text/plain; charset=utf-8',
		'Content-Transfer-Encoding: base64',
		'',
		b64(lines.join('\r\n')).replace(/.{76}/g, '$&\r\n'),
	].join('\r\n');
	try {
		await env.SUPPORT_MAIL.send(new EmailMessage(SUPPORT_FROM, env.SUPPORT_INBOX, raw));
	} catch (e) {
		console.error('support email failed', e && e.message);
		return json(502, { error: 'could not send email' });
	}
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
		if (url.pathname === '/api/support') {
			return request.method === 'POST' ? handleSupport(request, env) : empty(405);
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
