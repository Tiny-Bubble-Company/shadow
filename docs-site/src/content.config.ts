import { defineCollection } from 'astro:content';
import { z } from 'astro/zod';
import { docsLoader } from '@astrojs/starlight/loaders';
import { docsSchema } from '@astrojs/starlight/schema';

export const collections = {
	docs: defineCollection({
		loader: docsLoader(),
		schema: docsSchema({
			extend: z.object({
				/**
				 * Questions and answers shown at the bottom of the page and emitted as FAQPage
				 * JSON-LD. Answers may use inline Markdown: links, `code`, **bold**, *italic*.
				 */
				faq: z
					.array(z.object({ q: z.string().min(1), a: z.string().min(1) }))
					.optional(),
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
			}),
		}),
	}),
};
