import { defineCollection } from 'astro:content';
import { z } from 'astro/zod';
import { glob } from 'astro/loaders';
import { docsLoader } from '@astrojs/starlight/loaders';
import { docsSchema } from '@astrojs/starlight/schema';

const faq = z.array(z.object({ q: z.string().min(1), a: z.string().min(1) }));

/**
 * Research articles (src/content/research/*.md), published at /research/<slug>/ by
 * src/pages/research/[...slug].astro inside the same Starlight page shell as the docs.
 */
const researchSchema = z.object({
	/** Visible headline (the page's only H1). */
	title: z.string(),
	/** The complete <title> for search results, used verbatim. 50–60 characters. */
	seoTitle: z.string().max(60).optional(),
	/** Dek under the headline, the meta description and the index card text. At most 160 characters. */
	description: z.string().max(160),
	publishedDate: z.coerce.date(),
	updatedDate: z.coerce.date().optional(),
	readingTime: z.number().optional(),
	/** Topic labels from src/data/research-topics.ts. */
	topics: z.array(z.string()).default([]),
	author: z.string().default('Shadow Research Team'),
	/** Highlights shown in a box above the article (inline HTML allowed). */
	keyFindings: z.array(z.string()).default([]),
	/** Methodology and caveats (Markdown: paragraphs, bold, lists). */
	methodology: z.string().optional(),
	draft: z.boolean().default(false),
	/** Open Graph / Twitter image: absolute URL or a path under public/. */
	ogImage: z.string().optional(),
	keywords: z.array(z.string()).default([]),
	faq: faq.default([]),
	series: z.string().optional(),
	seriesNumber: z.number().optional(),
	cta: z.object({ label: z.string(), href: z.string() }).optional(),
	/** Slugs of related research articles; defaults to articles sharing a topic. */
	related: z.array(z.string()).default([]),
});

export type ResearchData = z.infer<typeof researchSchema>;

export const collections = {
	docs: defineCollection({
		loader: docsLoader(),
		schema: docsSchema({
			extend: z.object({
				/**
				 * Questions and answers shown at the bottom of the page and emitted as FAQPage
				 * JSON-LD. Answers may use inline Markdown: links, `code`, **bold**, *italic*.
				 */
				faq: faq.optional(),
				/**
				 * The complete <title> (and og:title) for search results, used verbatim, so the
				 * visible H1 (`title`) can stay descriptive. Aim for 50–60 characters, primary
				 * keyword first, "Shadow" at most once. Without it, the title is built from
				 * `title` in src/seo/middleware.ts.
				 */
				seoTitle: z.string().max(60).optional(),
				/** Rendered as <meta name="keywords">. */
				keywords: z.array(z.string()).optional(),
				/** Open Graph / Twitter image for this page: absolute URL or a path under public/. */
				ogImage: z.string().optional(),
				/**
				 * Set by the research pages (never in docs Markdown): turns the page's JSON-LD into
				 * an Article with these dates and author, and the H1 into the article hero.
				 */
				article: z
					.object({
						published: z.coerce.date(),
						modified: z.coerce.date().optional(),
						author: z.string(),
						readingTime: z.number().optional(),
						series: z.string().optional(),
						topics: z.array(z.object({ label: z.string(), href: z.string() })).default([]),
					})
					.optional(),
			}),
		}),
	}),
	research: defineCollection({
		loader: glob({ pattern: '*.md', base: './src/content/research' }),
		schema: researchSchema,
	}),
};
