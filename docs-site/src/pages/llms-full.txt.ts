// /llms-full.txt — every docs page as Markdown in one file, with source URLs.
import type { APIRoute } from 'astro';
import { getEntry } from 'astro:content';
import { HOME_URL, SITE_DESCRIPTION } from '../site.mjs';
import { groupedPages, pageUrl, toPlainMarkdown, type DocEntry } from '../seo/docs-index';
import { inlineMarkdownToText } from '../seo/inline-md';

export const prerender = true;

function render(e: DocEntry, url: string): string {
	const data = e.data as DocEntry['data'] & { faq?: { q: string; a: string }[] };
	const parts = [`# ${data.title}`, '', `Source: ${url}`, ''];
	if (data.description) parts.push(`> ${data.description}`, '');
	parts.push(toPlainMarkdown(e.body ?? ''));
	if (data.faq?.length) {
		parts.push('', '## Frequently asked questions', '');
		for (const { q, a } of data.faq) parts.push(`### ${inlineMarkdownToText(q)}`, '', toPlainMarkdown(a), '');
	}
	return parts.join('\n').trim();
}

export const GET: APIRoute = async () => {
	const groups = await groupedPages();
	const home = await getEntry('docs', 'index');
	const chunks: string[] = [
		`# Shadow documentation\n\n> ${SITE_DESCRIPTION}\n\nSource: ${HOME_URL}\n\nThis file concatenates every page of the Shadow documentation as Markdown. Each page starts with a level-1 heading and its source URL.`,
	];
	if (home) chunks.push(render(home, HOME_URL));
	for (const g of groups) for (const e of g.entries) chunks.push(render(e, pageUrl(e)));
	return new Response(chunks.join('\n\n---\n\n') + '\n', {
		headers: { 'Content-Type': 'text/plain; charset=utf-8' },
	});
};
