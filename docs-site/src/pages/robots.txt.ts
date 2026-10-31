// /robots.txt — allow search engines and AI crawlers, point them at the sitemap.
// Crawlers only read robots.txt at the domain root, so build with BASE="" for it to apply.
import type { APIRoute } from 'astro';
import { HOME_URL } from '../site.mjs';

export const prerender = true;

const AGENTS = [
	'Googlebot',
	'Bingbot',
	'DuckDuckBot',
	'Applebot',
	'Applebot-Extended',
	'Google-Extended',
	'GPTBot',
	'OAI-SearchBot',
	'ChatGPT-User',
	'ClaudeBot',
	'Claude-Web',
	'Claude-SearchBot',
	'Claude-User',
	'anthropic-ai',
	'PerplexityBot',
	'Perplexity-User',
	'CCBot',
	'Meta-ExternalAgent',
	'cohere-ai',
	'MistralAI-User',
	'YouBot',
];

export const GET: APIRoute = () => {
	const lines = [
		'# Shadow documentation: open to search engines and AI crawlers.',
		`# LLM-friendly index: ${new URL('llms.txt', HOME_URL).href}`,
		'',
		...AGENTS.flatMap((ua) => [`User-agent: ${ua}`, 'Allow: /', '']),
		'User-agent: *',
		'Allow: /',
		'',
		`Sitemap: ${new URL('sitemap-index.xml', HOME_URL).href}`,
		'',
	];
	return new Response(lines.join('\n'), { headers: { 'Content-Type': 'text/plain; charset=utf-8' } });
};
