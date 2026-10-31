// Starlight route middleware: title template, Open Graph / Twitter gaps, keywords, and
// JSON-LD structured data (Organization, WebSite, SoftwareApplication, TechArticle,
// BreadcrumbList, FAQPage) for every docs page.

import { defineRouteMiddleware } from '@astrojs/starlight/route-data';
import type { HeadConfig } from '@astrojs/starlight/types';
import { getCollection } from 'astro:content';
import {
	HOME_URL,
	ORG_NAME,
	ORG_URL,
	RELEASES_URL,
	REPO_URL,
	SITE_DESCRIPTION,
	SITE_NAME,
	TAGLINE,
} from '../site.mjs';
import { fileDates } from './git-dates';
import { inlineMarkdownToText } from './inline-md';
import { idToSlug, sectionLabel } from './sections';

type HeadEntry = HeadConfig[number];

let slugSet: Promise<Map<string, string>> | undefined;
/** slug -> title, for breadcrumb intermediate levels. */
function knownPages() {
	slugSet ??= getCollection('docs').then(
		(entries) => new Map(entries.map((e) => [idToSlug(e.id), e.data.title as string])),
	);
	return slugSet;
}

const abs = (path: string) => new URL(path.replace(/^\//, ''), HOME_URL).href;
const pageUrl = (slug: string) => abs(slug ? `${slug}/` : '');
const ldJson = (data: unknown) => JSON.stringify(data).replace(/</g, '\\u003c');

function findMeta(head: HeadEntry[], key: 'name' | 'property', value: string) {
	return head.find((h) => h.tag === 'meta' && h.attrs?.[key] === value);
}
function setMeta(head: HeadEntry[], key: 'name' | 'property', value: string, content: string, overwrite = true) {
	const existing = findMeta(head, key, value);
	if (existing) {
		if (overwrite) existing.attrs = { ...existing.attrs, content };
	} else {
		head.push({ tag: 'meta', attrs: { [key]: value, content } });
	}
}

export const onRequest = defineRouteMiddleware(async (context) => {
	const route = context.locals.starlightRoute;
	const { entry, head } = route;
	const data = entry.data as typeof entry.data & {
		faq?: { q: string; a: string }[];
		keywords?: string[];
		ogImage?: string;
	};
	const slug = idToSlug(route.id ?? '');
	const isHome = slug === '';
	const is404 = slug === '404' || entry.id === '404';

	// Title: "{page} · Shadow — open-source privacy layer for AI". Long page titles
	// (already keyword-rich, e.g. comparisons) drop the tagline so search results do not
	// truncate them. The home page's frontmatter title is used as is.
	const fullTitle = isHome
		? data.title
		: data.title.length > 48
			? `${data.title} · ${SITE_NAME}`
			: `${data.title} · ${SITE_NAME} — ${TAGLINE}`;
	const titleTag = head.find((h) => h.tag === 'title');
	if (titleTag) titleTag.content = fullTitle;
	else head.push({ tag: 'title', content: fullTitle });

	const description = data.description || SITE_DESCRIPTION;
	const canonicalHref =
		(head.find((h) => h.tag === 'link' && h.attrs?.rel === 'canonical')?.attrs?.href as string | undefined) ??
		pageUrl(slug);

	// Open Graph and Twitter.
	const image = data.ogImage ? (/^https?:/.test(data.ogImage) ? data.ogImage : abs(data.ogImage)) : abs('og.png');
	setMeta(head, 'property', 'og:title', isHome ? data.title : `${data.title} · ${SITE_NAME}`);
	setMeta(head, 'property', 'og:type', isHome ? 'website' : 'article');
	setMeta(head, 'property', 'og:image', image, false);
	if (!data.ogImage) {
		setMeta(head, 'property', 'og:image:width', '1200', false);
		setMeta(head, 'property', 'og:image:height', '630', false);
		setMeta(head, 'property', 'og:image:type', 'image/png', false);
	}
	setMeta(head, 'property', 'og:image:alt', `${SITE_NAME}: ${TAGLINE}`, false);
	setMeta(head, 'name', 'twitter:card', 'summary_large_image');
	setMeta(head, 'name', 'twitter:title', isHome ? data.title : `${data.title} · ${SITE_NAME}`, false);
	setMeta(head, 'name', 'twitter:description', description, false);
	setMeta(head, 'name', 'twitter:image', image, false);
	if (data.keywords?.length) setMeta(head, 'name', 'keywords', data.keywords.join(', '));
	if (is404) setMeta(head, 'name', 'robots', 'noindex');

	// FAQ entries also appear in the on-page table of contents.
	if (data.faq?.length && route.toc && !route.toc.items.some((i) => i.slug === 'faq')) {
		route.toc.items.push({ depth: 2, slug: 'faq', text: 'Frequently asked questions', children: [] });
	}

	if (is404) return;

	// Structured data.
	const orgId = `${HOME_URL}#organization`;
	const siteId = `${HOME_URL}#website`;
	const appId = `${HOME_URL}#software`;
	const graph: Record<string, unknown>[] = [
		{
			'@type': 'Organization',
			'@id': orgId,
			name: ORG_NAME,
			url: ORG_URL,
			logo: { '@type': 'ImageObject', url: abs('logo.png'), width: 512, height: 512 },
			sameAs: [ORG_URL, REPO_URL],
		},
		{
			'@type': 'WebSite',
			'@id': siteId,
			url: HOME_URL,
			name: `${SITE_NAME} documentation`,
			alternateName: SITE_NAME,
			description: SITE_DESCRIPTION,
			inLanguage: 'en',
			publisher: { '@id': orgId },
		},
		{
			'@type': 'SoftwareApplication',
			'@id': appId,
			name: SITE_NAME,
			description: SITE_DESCRIPTION,
			url: HOME_URL,
			applicationCategory: 'SecurityApplication',
			applicationSubCategory: 'Data loss prevention, PII redaction',
			operatingSystem: 'macOS, Linux, Windows',
			softwareRequirements: 'None (single static binary); Rust 1.88+ to build from source',
			license: 'https://www.apache.org/licenses/LICENSE-2.0',
			isAccessibleForFree: true,
			offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
			downloadUrl: RELEASES_URL,
			installUrl: pageUrl('getting-started/installation'),
			softwareHelp: { '@type': 'CreativeWork', url: HOME_URL },
			sameAs: [REPO_URL],
			image: abs('og.png'),
			publisher: { '@id': orgId },
			author: { '@id': orgId },
		},
	];

	// Breadcrumbs from the URL path; intermediate levels only when a page exists there.
	const pages = await knownPages();
	const crumbs: { name: string; item: string }[] = [{ name: SITE_NAME, item: HOME_URL }];
	if (!isHome) {
		const parts = slug.split('/');
		for (let i = 1; i < parts.length; i++) {
			const sub = parts.slice(0, i).join('/');
			if (pages.has(sub)) crumbs.push({ name: i === 1 ? sectionLabel(sub) : pages.get(sub)!, item: pageUrl(sub) });
		}
		crumbs.push({ name: data.title, item: canonicalHref });
	}
	if (crumbs.length > 1) {
		graph.push({
			'@type': 'BreadcrumbList',
			'@id': `${canonicalHref}#breadcrumb`,
			itemListElement: crumbs.map((c, i) => ({
				'@type': 'ListItem',
				position: i + 1,
				name: c.name,
				item: c.item,
			})),
		});
	}

	const dates = fileDates(entry.filePath);
	if (isHome) {
		graph.push({
			'@type': 'WebPage',
			'@id': `${canonicalHref}#webpage`,
			url: canonicalHref,
			name: data.title,
			description,
			inLanguage: 'en',
			isPartOf: { '@id': siteId },
			about: { '@id': appId },
			mainEntity: { '@id': appId },
			primaryImageOfPage: { '@type': 'ImageObject', url: image },
			dateModified: dates.modified,
		});
	} else {
		graph.push({
			'@type': 'TechArticle',
			'@id': `${canonicalHref}#article`,
			headline: data.title,
			name: data.title,
			description,
			url: canonicalHref,
			mainEntityOfPage: canonicalHref,
			inLanguage: 'en',
			datePublished: dates.published,
			dateModified: dates.modified,
			image,
			isPartOf: { '@id': siteId },
			about: { '@id': appId },
			author: { '@id': orgId },
			publisher: { '@id': orgId },
			...(data.keywords?.length ? { keywords: data.keywords.join(', ') } : {}),
			...(crumbs.length > 1 ? { breadcrumb: { '@id': `${canonicalHref}#breadcrumb` } } : {}),
		});
	}

	if (data.faq?.length) {
		graph.push({
			'@type': 'FAQPage',
			'@id': `${canonicalHref}#faq`,
			url: canonicalHref,
			inLanguage: 'en',
			isPartOf: { '@id': siteId },
			mainEntity: data.faq.map(({ q, a }) => ({
				'@type': 'Question',
				name: inlineMarkdownToText(q),
				acceptedAnswer: { '@type': 'Answer', text: inlineMarkdownToText(a) },
			})),
		});
	}

	head.push({
		tag: 'script',
		attrs: { type: 'application/ld+json' },
		content: ldJson({ '@context': 'https://schema.org', '@graph': graph }),
	});
});
