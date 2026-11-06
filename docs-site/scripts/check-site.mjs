// Checks the built site in dist/: one H1 per page, unique <title> and meta description,
// canonical URL, Open Graph and Twitter tags with images that exist, valid JSON-LD, alt
// text on images, internal links that resolve, and a sitemap that lists only live,
// indexable pages. Exits non-zero on any error.
//
//   pnpm build && node scripts/check-site.mjs

import { readFileSync, existsSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const DIST = fileURLToPath(new URL('../dist/', import.meta.url));
const { SITE, BASE } = await import('../src/site.mjs');
const ORIGIN = new URL(SITE).origin;
const base = (BASE || '').replace(/\/$/, '');

// Old URLs answered with a 301 by worker/index.js: links to them are allowed but reported.
const workerSrc = readFileSync(new URL('../worker/index.js', import.meta.url), 'utf8');
const REDIRECTS = Object.fromEntries(
	[...workerSrc.matchAll(/'(\/[^']+\/)':\s*'(\/[^']+\/)'/g)].map((m) => [m[1], m[2]]),
);

const errors = [];
const warnings = [];
const err = (page, msg) => errors.push(`${page}: ${msg}`);
const warn = (page, msg) => warnings.push(`${page}: ${msg}`);

function walk(dir, out = []) {
	for (const name of readdirSync(dir)) {
		const p = join(dir, name);
		if (statSync(p).isDirectory()) walk(p, out);
		else if (name.endsWith('.html')) out.push(p);
	}
	return out;
}

const decode = (s) =>
	s
		.replace(/&amp;/g, '&')
		.replace(/&quot;/g, '"')
		.replace(/&#39;|&#x27;/g, "'")
		.replace(/&lt;/g, '<')
		.replace(/&gt;/g, '>');
const attr = (tag, name) => {
	const m = tag.match(new RegExp(`\\s${name}=(?:"([^"]*)"|'([^']*)'|([^\\s>]+))`, 'i'));
	return m ? decode(m[1] ?? m[2] ?? m[3] ?? '') : undefined;
};
const meta = (html, key, value) => {
	for (const m of html.matchAll(/<meta\b[^>]*>/gi)) {
		if (attr(m[0], key) === value) return attr(m[0], 'content');
	}
	return undefined;
};

/** URL path ('/research/x/') -> file in dist, or undefined. */
function resolvePath(path) {
	let p = decodeURIComponent(path.split('#')[0].split('?')[0]);
	if (base && p.startsWith(base)) p = p.slice(base.length) || '/';
	const candidates = p.endsWith('/') ? [`${p}index.html`] : [p, `${p}/index.html`, `${p}.html`];
	return candidates.map((c) => join(DIST, c)).find((f) => existsSync(f) && statSync(f).isFile());
}

const files = walk(DIST);
const titles = new Map();
const descriptions = new Map();
const indexable = new Set();
const noindex = new Set();

for (const file of files) {
	const rel = '/' + relative(DIST, file).replace(/index\.html$/, '').replace(/\\/g, '/');
	if (rel.startsWith('/_astro') || rel.startsWith('/pagefind')) continue;
	const html = readFileSync(file, 'utf8');
	const is404 = rel === '/404.html';
	const robots = meta(html, 'name', 'robots') ?? '';
	const isNoindex = /noindex/.test(robots) || is404;
	if (/http-equiv="refresh"/i.test(html)) continue; // redirect stub

	const h1s = html.match(/<h1\b/gi)?.length ?? 0;
	if (h1s !== 1) err(rel, `${h1s} <h1> elements`);

	const title = decode(html.match(/<title>([^<]*)<\/title>/i)?.[1] ?? '').trim();
	if (!title) err(rel, 'missing <title>');
	else {
		if (title.length > 65) warn(rel, `title is ${title.length} chars: "${title}"`);
		if (!isNoindex) titles.set(title, [...(titles.get(title) ?? []), rel]);
	}

	const desc = meta(html, 'name', 'description');
	if (!desc) err(rel, 'missing meta description');
	else {
		if (desc.length > 165) warn(rel, `description is ${desc.length} chars`);
		if (!isNoindex) descriptions.set(desc, [...(descriptions.get(desc) ?? []), rel]);
	}

	const canonical = html.match(/<link\b[^>]*rel="canonical"[^>]*>/i);
	if (!is404) {
		if (!canonical) err(rel, 'missing canonical');
		else {
			const href = attr(canonical[0], 'href');
			const expected = new URL(`${base}${rel}`, ORIGIN).href;
			if (href !== expected) err(rel, `canonical ${href} != ${expected}`);
		}
	}

	for (const [k, v] of [
		['property', 'og:title'],
		['property', 'og:description'],
		['property', 'og:image'],
		['property', 'og:url'],
		['property', 'og:type'],
		['name', 'twitter:card'],
		['name', 'twitter:image'],
	]) {
		if (!is404 && !meta(html, k, v)) err(rel, `missing ${v}`);
	}
	for (const key of ['og:image', 'twitter:image']) {
		const img = meta(html, key === 'og:image' ? 'property' : 'name', key);
		if (img?.startsWith(ORIGIN) && !resolvePath(new URL(img).pathname)) err(rel, `${key} ${img} not in dist`);
	}

	for (const m of html.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/gi)) {
		try {
			JSON.parse(m[1]);
		} catch {
			err(rel, 'invalid JSON-LD');
		}
	}
	const ldCount = html.match(/application\/ld\+json/g)?.length ?? 0;
	if (!is404 && ldCount === 0) err(rel, 'no JSON-LD');
	if (ldCount > 1) warn(rel, `${ldCount} JSON-LD blocks`);

	for (const m of html.matchAll(/<img\b[^>]*>/gi)) {
		if (attr(m[0], 'alt') === undefined) err(rel, `<img> without alt: ${attr(m[0], 'src')}`);
		const src = attr(m[0], 'src');
		if (src && src.startsWith('/') && !resolvePath(src)) err(rel, `broken image ${src}`);
	}

	for (const m of html.matchAll(/<a\b[^>]*\shref="([^"]+)"/gi)) {
		let href = decode(m[1]);
		if (href.startsWith(ORIGIN)) href = href.slice(ORIGIN.length) || '/';
		if (!href.startsWith('/') || href.startsWith('//')) continue;
		const path = href.split('#')[0].split('?')[0];
		if (!path) continue;
		if (/^\/(api|cdn-cgi)\//.test(path)) continue;
		const withSlash = path.endsWith('/') || /\.\w+$/.test(path) ? path : `${path}/`;
		if (REDIRECTS[withSlash.replace(base, '')]) {
			warn(rel, `links to redirected ${path}`);
			continue;
		}
		if (!resolvePath(path)) err(rel, `broken link ${href}`);
	}

	(isNoindex ? noindex : indexable).add(new URL(`${base}${rel}`, ORIGIN).href);
}

for (const [t, pages] of titles) if (pages.length > 1) err(pages.join(', '), `duplicate title "${t}"`);
for (const [d, pages] of descriptions) if (pages.length > 1) err(pages.join(', '), `duplicate description "${d.slice(0, 60)}…"`);

// Sitemap: every listed URL is a live, indexable page; every indexable page is listed.
const sitemapUrls = new Set();
for (const f of readdirSync(DIST).filter((n) => /^sitemap-\d+\.xml$/.test(n))) {
	for (const m of readFileSync(join(DIST, f), 'utf8').matchAll(/<loc>([^<]+)<\/loc>/g)) sitemapUrls.add(m[1]);
}
for (const u of sitemapUrls) {
	if (noindex.has(u)) err('sitemap', `lists noindex page ${u}`);
	else if (!indexable.has(u)) err('sitemap', `lists non-page ${u}`);
}
for (const u of indexable) if (!sitemapUrls.has(u) && !u.endsWith('/404.html')) err('sitemap', `missing ${u}`);

// Redirect targets exist; redirect sources are no longer built.
for (const [from, to] of Object.entries(REDIRECTS)) {
	if (!resolvePath(to)) err('worker', `redirect ${from} -> ${to}: target missing`);
	if (resolvePath(from)) err('worker', `redirect source ${from} is still built (shadowed by the redirect)`);
}

const robots = existsSync(join(DIST, 'robots.txt')) ? readFileSync(join(DIST, 'robots.txt'), 'utf8') : '';
if (!/Sitemap:\s*\S+sitemap-index\.xml/.test(robots)) err('robots.txt', 'no Sitemap line');

for (const w of warnings) console.warn(`warn  ${w}`);
for (const e of errors) console.error(`error ${e}`);
console.log(`\n${files.length} HTML files, ${sitemapUrls.size} sitemap URLs, ${errors.length} errors, ${warnings.length} warnings`);
process.exit(errors.length ? 1 : 0);
