// Content collection configuration for Shadow documentation site.
// Starlight manages the 'docs' collection; this adds the 'research' collection for editorial
// articles outside the docs sidebar.

import { defineCollection, z } from 'astro:content';
import { glob } from 'astro/loaders';
import { docsLoader } from '@astrojs/starlight/loaders';
import { docsSchema } from '@astrojs/starlight/schema';

// Topic taxonomy for scalable long-tail content architecture.
// Primary topics are broad categories; secondary topics enable specific tool/provider combinations.
// Example: primary "AI Coding Tools", secondary ["Cursor", "Claude Code", "Privacy Mode"]
const researchSchema = z.object({
	title: z.string().describe('Headline for the article'),
	seoTitle: z.string().optional().describe('Custom <title> if different from headline'),
	description: z.string().describe('Dek / summary shown on index and in meta'),
	publishedDate: z.coerce.date().describe('Publication date'),
	updatedDate: z.coerce.date().optional().describe('Last updated date'),
	readingTime: z.number().optional().describe('Estimated reading time in minutes'),
	// Primary topic categories (broad): AI Privacy, AI Coding Tools, Enterprise AI, etc.
	topics: z.array(z.string()).default([]).describe('Primary topic categories for filtering'),
	// Secondary topics for long-tail (specific): tool names, provider names, specific features
	tools: z.array(z.string()).default([]).describe('Specific AI tools covered (Cursor, ChatGPT, Claude Code, etc.)'),
	providers: z.array(z.string()).default([]).describe('AI providers covered (OpenAI, Anthropic, Google, etc.)'),
	audiences: z.array(z.string()).default([]).describe('Target audiences (developers, CISOs, IT admins, etc.)'),
	author: z.string().default('Shadow Research Team').describe('Author name'),
	keyFindings: z.array(z.string()).default([]).describe('Bullet-point highlights/key findings'),
	methodology: z.string().optional().describe('Methodology and caveats section (Markdown)'),
	draft: z.boolean().default(false).describe('Draft articles are not published'),
	ogImage: z.string().optional().describe('Custom Open Graph image path'),
	keywords: z.array(z.string()).default([]).describe('SEO keywords'),
	faq: z
		.array(z.object({ q: z.string(), a: z.string() }))
		.default([])
		.describe('FAQ items for FAQ schema'),
	series: z.string().optional().describe('Series name (e.g. "AI Leak Report")'),
	seriesNumber: z.number().optional().describe('Episode number in series'),
	canonical: z.string().optional().describe('Canonical URL if different from default'),
	cta: z
		.object({
			label: z.string(),
			href: z.string(),
		})
		.optional()
		.describe('Call-to-action button'),
	// Related articles for internal linking
	related: z.array(z.string()).default([]).describe('Slugs of related research articles'),
});

export type ResearchFrontmatter = z.infer<typeof researchSchema>;

export const collections = {
	docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
	research: defineCollection({
		loader: glob({ pattern: '**/*.md', base: new URL('./research/', import.meta.url) }),
		schema: researchSchema,
	}),
};
