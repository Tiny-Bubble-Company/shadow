// RSS feed for Shadow Research articles.
import type { APIRoute } from 'astro';
import { getResearch } from '../../data/research';
import { HOME_URL, SITE_NAME, SITE_DESCRIPTION, ORG_NAME } from '../../site.mjs';

export const prerender = true;

const escapeXml = (text: string) =>
	text
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
		.replace(/'/g, '&apos;');

export const GET: APIRoute = async () => {
	const articles = await getResearch();

	const now = new Date().toUTCString();
	const items = articles
		.map((article) => {
			const url = `${HOME_URL}research/${article.id}/`;
			const pubDate = article.data.publishedDate.toUTCString();
			const categories = article.data.topics.map((t) => `<category>${escapeXml(t)}</category>`).join('');
			return `
    <item>
      <title>${escapeXml(article.data.title)}</title>
      <link>${url}</link>
      <guid isPermaLink="true">${url}</guid>
      <pubDate>${pubDate}</pubDate>
      <description>${escapeXml(article.data.description)}</description>
      <dc:creator>${escapeXml(article.data.author)}</dc:creator>
      ${categories}
    </item>`;
		})
		.join('');

	const rss = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom" xmlns:dc="http://purl.org/dc/elements/1.1/">
  <channel>
    <title>${SITE_NAME} Research</title>
    <link>${HOME_URL}research/</link>
    <description>Evidence-based research on AI data privacy and what developer tools send to model providers, from the ${SITE_NAME} Research Team.</description>
    <language>en</language>
    <lastBuildDate>${now}</lastBuildDate>
    <managingEditor>support@getshadow.dev (${ORG_NAME})</managingEditor>
    <webMaster>support@getshadow.dev (${ORG_NAME})</webMaster>
    <atom:link href="${HOME_URL}research/rss.xml" rel="self" type="application/rss+xml" />
    ${items}
  </channel>
</rss>`;

	return new Response(rss.trim(), {
		headers: {
			'Content-Type': 'application/rss+xml; charset=utf-8',
			'Cache-Control': 'public, max-age=3600',
		},
	});
};
