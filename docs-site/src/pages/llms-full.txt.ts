// /llms-full.txt — every docs page as Markdown in one file, with source URLs.
import type { APIRoute } from 'astro';
import { getEntry } from 'astro:content';
import { HOME_URL, SITE_DESCRIPTION } from '../site.mjs';
import { groupedPages, pageUrl, toPlainMarkdown, type DocEntry } from '../seo/docs-index';
import { inlineMarkdownToText } from '../seo/inline-md';
import { HOME_SUMMARY } from '../seo/home-summary';

export const prerender = true;

function render(e: DocEntry, url: string, extra?: string): string {
	const data = e.data as DocEntry['data'] & { faq?: { q: string; a: string }[] };
	const parts = [`# ${data.title}`, '', `Source: ${url}`, ''];
	if (data.description) parts.push(`> ${data.description}`, '');
	parts.push(toPlainMarkdown(e.body ?? ''));
	if (extra) parts.push('', extra);
	if (data.faq?.length) {
		parts.push('', '## Frequently asked questions', '');
		for (const { q, a } of data.faq) parts.push(`### ${inlineMarkdownToText(q)}`, '', toPlainMarkdown(a), '');
	}
	return parts.join('\n').trim();
}

interface ResearchPost {
	frontmatter: any;
	rawContent: () => string;
}

function renderResearch(post: ResearchPost, slug: string, url: string): string {
	const data = post.frontmatter;
	const body = post.rawContent?.() || '';
	const parts = [`# ${data.title}`, '', `Source: ${url}`, ''];
	if (data.series) parts.push(`Series: ${data.series}${data.seriesNumber ? ` #${data.seriesNumber}` : ''}`, '');
	parts.push(`Author: ${data.author || 'Shadow Research Team'}`, '');
	parts.push(`Published: ${new Date(data.publishedDate).toISOString().split('T')[0]}`, '');
	if (data.description) parts.push(`> ${data.description}`, '');
	if (data.keyFindings?.length) {
		parts.push('## Key findings', '');
		for (const finding of data.keyFindings) parts.push(`- ${finding}`);
		parts.push('');
	}
	parts.push(toPlainMarkdown(body));
	if (data.methodology) {
		parts.push('', '## Methodology', '', toPlainMarkdown(data.methodology));
	}
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
		`# Shadow documentation\n\n> ${SITE_DESCRIPTION}\n\nSource: ${HOME_URL}\n\nThis file concatenates every page of the Shadow documentation and research articles as Markdown. Each page starts with a level-1 heading and its source URL.`,
	];
	// The home page is mostly components; HOME_SUMMARY carries their content as text.
	if (home) chunks.push(render(home, HOME_URL, HOME_SUMMARY));
	for (const g of groups) for (const e of g.entries) chunks.push(render(e, pageUrl(e)));

	// Research articles
	try {
		const allPosts = import.meta.glob('../content/research/*.md', { eager: true });
		const research = Object.entries(allPosts)
			.filter(([_, post]: [string, any]) => !post.frontmatter?.draft)
			.map(([path, post]: [string, any]) => ({
				slug: path.split('/').pop()?.replace('.md', '') || '',
				post: post as ResearchPost,
				publishedDate: new Date(post.frontmatter.publishedDate),
			}));
		const sorted = research.sort((a, b) => b.publishedDate.valueOf() - a.publishedDate.valueOf());
		for (const { slug, post } of sorted) {
			const url = new URL(`research/${slug}/`, HOME_URL).href;
			chunks.push(renderResearch(post, slug, url));
		}
	} catch {
		// Research collection may not exist
	}

	return new Response(chunks.join('\n\n---\n\n') + '\n', {
		headers: { 'Content-Type': 'text/plain; charset=utf-8' },
	});
};
