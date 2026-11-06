// Site-wide navigation for the header mega menu, the footer and SiteNavigationElement
// JSON-LD. Section lists come from the docs collection, so new pages under use-cases/,
// industries/, company-size/, compare/, integrate/, concepts/ and reference/ show up
// automatically. Labels come from `sidebar.label`, else a short form of the title.

import { getCollection, type CollectionEntry } from 'astro:content';
import { REPO_URL, RELEASES_URL, SUPPORT_EMAIL } from '../site.mjs';
import { SECTIONS, idToSlug } from '../seo/sections';

type Entry = CollectionEntry<'docs'>;
export interface NavLink {
	label: string;
	href: string;
	/** Optional one-line description (used in the header panels). */
	description?: string;
	external?: boolean;
}
export interface NavGroup {
	id: string;
	label: string;
	/** Section landing page, if there is one. */
	href?: string;
	/** One-line pitch shown under the heading (Products menu). */
	tagline?: string;
	links: NavLink[];
}

const BASE = (import.meta.env.BASE_URL ?? '/').replace(/\/+$/, '');
/** Site-internal path ('/getting-started/quick-start/') with the configured base. */
export const sitePath = (path: string) => `${BASE}${path.startsWith('/') ? path : `/${path}`}`;
const slugPath = (slug: string) => sitePath(slug ? `/${slug}/` : '/');

const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);
const beforeColon = (t: string) => t.split(/:\s| — /)[0]!.trim();

/** Short, descriptive anchor text for a page. */
function labelFor(e: Entry, dir: string): string {
	const data = e.data as Entry['data'] & { sidebar?: { label?: string } };
	const title = data.title;
	const sidebarLabel = data.sidebar?.label;
	switch (dir) {
		case 'compare': {
			const name = sidebarLabel ?? beforeColon(title).replace(/^Shadow vs\.?\s+/i, '');
			if (!/^Shadow vs/i.test(title)) return name; // round-ups such as "AI DLP tools compared"
			return /alternative/i.test(title) ? `${name} alternative` : `Shadow vs ${name}`;
		}
		case 'industries': {
			const head = beforeColon(title)
				.replace(/^Secure\s+/i, '')
				.replace(/^(?:ChatGPT and |generative )?(?:LLM|AI|ChatGPT and AI|ChatGPT and LLM)\s+(?:use\s+)?(?:in|for|by)\s+(?:the\s+)?/i, '')
				.replace(/^(?:generative AI|AI)\s+(?:in|for)\s+(?:the\s+)?/i, '');
			return cap(head);
		}
		case 'use-cases':
			return cap(beforeColon(title).replace(/^How to\s+/i, ''));
		case 'company-size':
			return cap(beforeColon(title));
		default:
			return sidebarLabel && sidebarLabel !== 'Overview' ? sidebarLabel : beforeColon(title);
	}
}

const order = (dir: string, e: Entry) => {
	const name = idToSlug(e.id).slice(dir.length + 1);
	const manual = SECTIONS.find((s) => s.dir === dir)?.pages;
	const i = manual?.indexOf(name) ?? -1;
	if (i >= 0) return i;
	return ((e.data as { sidebar?: { order?: number } }).sidebar?.order ?? 1e6) + 1000;
};

let cache: Promise<Record<string, NavGroup>> | undefined;

/** Every docs section as a group of links, keyed by directory. */
export function sectionGroups(): Promise<Record<string, NavGroup>> {
	cache ??= getCollection('docs').then((all) => {
		const out: Record<string, NavGroup> = {};
		const pages = all.filter((e) => !e.data.draft && e.id !== 'index' && !/(^|\/)404$/.test(e.id));
		for (const s of SECTIONS) {
			const inDir = pages.filter((e) => e.id.split('/')[0] === s.dir);
			if (!inDir.length) continue;
			const index = inDir.find((e) => idToSlug(e.id) === s.dir);
			const children = inDir
				.filter((e) => e !== index)
				.sort((a, b) => order(s.dir, a) - order(s.dir, b) || labelFor(a, s.dir).localeCompare(labelFor(b, s.dir)));
			out[s.dir] = {
				id: s.dir,
				label: s.label,
				href: index ? slugPath(s.dir) : undefined,
				links: children.map((e) => ({
					label: labelFor(e, s.dir),
					href: slugPath(idToSlug(e.id)),
					description: e.data.description,
				})),
			};
		}
		return out;
	});
	return cache;
}

export const RESOURCE_LINKS: NavLink[] = [
	{ label: `Email support: ${SUPPORT_EMAIL}`, href: `mailto:${SUPPORT_EMAIL}` },
	{ label: 'GitHub repository', href: REPO_URL, external: true },
	{ label: 'Releases and downloads', href: RELEASES_URL, external: true },
	{ label: 'Apache-2.0 licence', href: `${REPO_URL}/blob/main/LICENSE`, external: true },
	{ label: 'Contribute', href: sitePath('/project/contributing/') },
	{ label: 'Report a vulnerability', href: `${REPO_URL}/blob/main/SECURITY.md`, external: true },
	{ label: 'Code of conduct', href: `${REPO_URL}/blob/main/CODE_OF_CONDUCT.md`, external: true },
	{ label: 'Open an issue', href: `${REPO_URL}/issues`, external: true },
	{ label: 'llms.txt (docs index for AI)', href: sitePath('/llms.txt') },
	{ label: 'llms-full.txt (all docs as Markdown)', href: sitePath('/llms-full.txt') },
	{ label: 'Sitemap', href: sitePath('/sitemap-index.xml') },
];

/** The top-level menu: what the header shows and what SiteNavigationElement lists. */
export async function mainNav() {
	const g = await sectionGroups();
	const pick = (dir: string) => g[dir] ?? { id: dir, label: dir, links: [] };
	const hub: NavGroup = {
		id: 'hub',
		label: 'Shadow Hub',
		href: sitePath('/hub/'),
		links: [
			{ label: 'Shadow Hub for teams', href: sitePath('/hub/'), description: 'See what your team sends to AI, per user and tool' },
			{ label: 'Pricing', href: sitePath('/pricing/'), description: 'Core is free; Hub is per monitored user' },
			{ label: 'Trust and data', href: sitePath('/trust/'), description: 'What Hub receives, stores and never sees' },
			{ label: 'Connect a machine', href: sitePath('/hub/connect/'), description: 'shadow hub connect, and how to disconnect' },
			{ label: 'Teams and organisations', href: sitePath('/integrate/organisations/'), description: 'Invites, sharing levels, the managed notice' },
			{ label: 'Compare AI DLP tools', href: sitePath('/compare/ai-dlp/'), description: 'Endpoint DLP vs browser extension vs LLM proxy' },
		],
	};
	const connect: NavGroup = {
		id: 'connect',
		label: 'Shadow Connect',
		href: sitePath('/connect/'),
		links: [
			{ label: 'Shadow Connect overview', href: sitePath('/connect/'), description: 'Shadow for other platforms and agents' },
			{ label: 'Privacy API', href: sitePath('/connect/privacy-api/'), description: 'Scan, protect and restore over HTTP' },
			{ label: 'MCP server', href: sitePath('/connect/mcp/'), description: 'Let any agent mask and restore data' },
			{ label: 'Evidence API', href: sitePath('/connect/evidence-api/'), description: 'Controls mapped to SOC 2, ISO 27001, ISO 42001' },
			{ label: 'Compliance platforms', href: sitePath('/connect/compliance-platforms/'), description: 'Vanta, Drata, Secureframe, Sprinto and more' },
			{ label: 'Meridian', href: sitePath('/connect/meridian/'), description: 'Built-in privacy and evidence' },
		],
	};
	const product: NavGroup = {
		id: 'product',
		label: 'Shadow Core',
		href: sitePath('/'),
		links: [
			{ label: 'What is Shadow', href: sitePath('/getting-started/introduction/'), description: 'Reversible fakes instead of blocking' },
			{ label: 'How it works', href: sitePath('/concepts/how-it-works/'), description: 'Intercept, detect, translate, restore' },
			{ label: 'Features', href: sitePath('/#features'), description: 'Vault, detection, streams, tool calls, files' },
			{ label: 'Activity dashboard', href: sitePath('/#activity'), description: 'Menu bar, web dashboard and shadow top' },
			{ label: 'Benchmarks', href: sitePath('/benchmarks/'), description: 'Leak rates vs Presidio, GLiNER, LLM Guard' },
			{ label: 'Security model', href: sitePath('/reference/security/'), description: 'What Shadow protects, and what it does not' },
			{ label: 'Supported providers', href: sitePath('/reference/providers/'), description: 'OpenAI, Anthropic, Gemini, Bedrock and more' },
			{ label: 'Roadmap', href: sitePath('/project/roadmap/') },
			{ label: 'Changelog', href: sitePath('/project/changelog/') },
		],
	};
	// The header's Products menu: one curated column per product, with a short pitch.
	const products: NavGroup[] = [
		{
			id: 'core',
			label: 'Shadow Core',
			href: sitePath('/getting-started/introduction/'),
			tagline: 'Free, open-source privacy layer on every machine',
			links: [
				{ label: 'How it works', href: sitePath('/concepts/how-it-works/'), description: 'Reversible fakes instead of blocking' },
				{ label: 'Quick start', href: sitePath('/getting-started/quick-start/'), description: 'Install in a minute, no account' },
				{ label: 'Supported providers', href: sitePath('/reference/providers/'), description: 'OpenAI, Anthropic, Gemini, Bedrock and more' },
				{ label: 'Benchmarks', href: sitePath('/benchmarks/'), description: 'Leak rates vs Presidio, GLiNER, LLM Guard' },
				{ label: 'Security model', href: sitePath('/reference/security/'), description: 'What Shadow protects, and what it does not' },
			],
		},
		{
			id: 'hub',
			label: 'Shadow Hub',
			href: sitePath('/hub/'),
			tagline: 'Team visibility into what goes to AI',
			links: [
				{ label: 'Hub for teams', href: sitePath('/hub/'), description: 'Usage per user and tool, as counts only' },
				{ label: 'Connect a machine', href: sitePath('/hub/connect/'), description: 'shadow hub connect, and how to leave' },
				{ label: 'Teams and organisations', href: sitePath('/integrate/organisations/'), description: 'Invites, sharing levels, the managed notice' },
				{ label: 'Trust and data', href: sitePath('/trust/'), description: 'What Hub receives, stores and never sees' },
			],
		},
		{
			id: 'connect',
			label: 'Shadow Connect',
			href: sitePath('/connect/'),
			tagline: 'Shadow inside other platforms and agents',
			links: [
				{ label: 'Privacy API', href: sitePath('/connect/privacy-api/'), description: 'Scan, protect and restore over HTTP' },
				{ label: 'MCP server', href: sitePath('/connect/mcp/'), description: 'Let any agent mask and restore data' },
				{ label: 'Evidence API', href: sitePath('/connect/evidence-api/'), description: 'Controls mapped to SOC 2, ISO 27001, ISO 42001' },
				{ label: 'Compliance platforms', href: sitePath('/connect/compliance-platforms/'), description: 'Vanta, Drata, Secureframe, Sprinto and more' },
				{ label: 'Meridian', href: sitePath('/connect/meridian/'), description: 'Built-in privacy and evidence' },
			],
		},
	];
	const docs = {
		id: 'docs',
		label: 'Docs',
		href: sitePath('/getting-started/introduction/'),
		columns: [pick('getting-started'), pick('integrate'), pick('connect'), pick('concepts'), pick('reference')],
	};
	const solutions = {
		id: 'solutions',
		label: 'Solutions',
		href: pick('use-cases').href,
		columns: [pick('use-cases'), pick('company-size'), pick('industries')],
	};
	const industries = pick('industries');
	const compare = pick('compare');

	// Research section: editorial articles outside the docs sidebar
	// Static links since Astro.glob doesn't work in regular TS modules
	const research: NavGroup = {
		id: 'research',
		label: 'Research',
		href: sitePath('/research/'),
		links: [
			{ label: 'AI Leak Report #1', href: sitePath('/research/ai-leak-report-1/'), description: '122k+ values protected on one laptop' },
			{ label: 'Does Cursor train on my code?', href: sitePath('/research/does-cursor-train-on-my-code/'), description: 'Privacy controls and residual risks' },
			{ label: 'Private ChatGPT', href: sitePath('/research/private-chatgpt/'), description: 'Use AI without sharing your data' },
			{ label: 'Is ChatGPT safe for company data?', href: sitePath('/research/is-chatgpt-safe-for-company-data/'), description: 'What happens to your prompts' },
			{ label: 'Browse all research', href: sitePath('/research/'), description: 'AI privacy research and analysis' },
		],
	};

	return { product, hub, connect, products, docs, solutions, industries, compare, research };
}
