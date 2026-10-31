// /llms.txt — an index of the docs for LLMs and answer engines (https://llmstxt.org).
import type { APIRoute } from 'astro';
import { HOME_URL, REPO_URL, SITE_DESCRIPTION } from '../site.mjs';
import { groupedPages, pageUrl } from '../seo/docs-index';

export const prerender = true;

export const GET: APIRoute = async () => {
	const groups = await groupedPages();
	const lines: string[] = [
		'# Shadow',
		'',
		`> ${SITE_DESCRIPTION}`,
		'',
		'Shadow is a single Rust binary (macOS, Linux, Windows) licensed under Apache-2.0. It runs on the endpoint as an HTTPS proxy for every tool, as a gateway base URL for SDKs, or in-process as the `shadow-core` Rust library. It detects personal data, customer names, credentials and machine identifiers in outbound AI requests (OpenAI, Anthropic, Google Gemini, Azure OpenAI, OpenRouter, Mistral, Groq and more), replaces them with format-preserving fakes drawn from a persistent persona (fake emails stay emails, fake card numbers still pass Luhn, fake IBANs still pass mod-97), keeps the mapping in an encrypted local vault, and restores the real values in responses, including streamed responses and tool-call arguments. Unlike DLP that blocks, it keeps AI tools useful. There is no cloud service, account or telemetry.',
		'',
		'Benchmark (held-out test set): Shadow\'s built-in rules leaked 3.08% of sensitive values at about 7.5 ms per document with a 100% exact round trip, and 1.35% with the Presidio detector added; standalone, OpenAI Privacy Filter leaked 11.01%, GLiNER multi PII 13.28% and Microsoft Presidio 20.74%.',
		'',
		`- Docs home: ${HOME_URL}`,
		'- Support: support@getshadow.dev',
		`- Benchmarks: ${new URL('benchmarks/', HOME_URL).href}`,
		`- Source code: ${REPO_URL}`,
		`- Full documentation as one Markdown file: ${new URL('llms-full.txt', HOME_URL).href}`,
		'',
	];
	const optional: typeof groups = [];
	for (const g of groups) {
		if (g.optional) {
			optional.push(g);
			continue;
		}
		lines.push(`## ${g.label}`, '');
		for (const e of g.entries) lines.push(`- [${e.data.title}](${pageUrl(e)}): ${e.data.description ?? ''}`.trimEnd());
		lines.push('');
	}
	if (optional.length) {
		lines.push('## Optional', '');
		for (const g of optional)
			for (const e of g.entries) lines.push(`- [${e.data.title}](${pageUrl(e)}): ${e.data.description ?? ''}`.trimEnd());
		lines.push('');
	}
	return new Response(lines.join('\n'), { headers: { 'Content-Type': 'text/plain; charset=utf-8' } });
};
