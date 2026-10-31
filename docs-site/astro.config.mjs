// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import sitemap from '@astrojs/sitemap';
import { unified } from '@astrojs/markdown-remark';
import rehypeBaseLinks from './src/plugins/rehype-base-links.mjs';
import { SITE, BASE, REPO_URL, SITE_DESCRIPTION } from './src/site.mjs';

// Where the site is published comes from the SITE and BASE environment variables
// (see src/site.mjs). Default: https://tiny-bubble-company.github.io/shadow/

// https://astro.build/config
export default defineConfig({
	site: SITE,
	base: BASE || '/',
	trailingSlash: 'ignore',
	markdown: {
		processor: unified({ rehypePlugins: [[rehypeBaseLinks, { base: BASE }]] }),
	},
	integrations: [
		// Starlight would add this automatically; listing it pins the version and lets us
		// keep the 404 page out of the sitemap.
		sitemap({
			filter: (page) => !/\/404\/?$/.test(page),
		}),
		starlight({
			title: 'Shadow',
			description: SITE_DESCRIPTION,
			logo: { src: './src/assets/logo.svg', alt: 'Shadow' },
			favicon: '/favicon.svg',
			social: [{ icon: 'github', label: 'GitHub', href: REPO_URL }],
			editLink: { baseUrl: `${REPO_URL}/edit/main/docs-site/` },
			lastUpdated: false,
			customCss: ['./src/styles/custom.css'],
			routeMiddleware: './src/seo/middleware.ts',
			components: {
				MarkdownContent: './src/components/MarkdownContent.astro',
			},
			head: [
				{ tag: 'meta', attrs: { name: 'theme-color', content: '#7C6CFF' } },
				{
					tag: 'link',
					attrs: {
						rel: 'alternate',
						type: 'text/plain',
						title: 'llms.txt',
						href: `${BASE}/llms.txt`,
					},
				},
				{
					tag: 'link',
					attrs: {
						rel: 'alternate',
						type: 'text/plain',
						title: 'llms-full.txt',
						href: `${BASE}/llms-full.txt`,
					},
				},
			],
			sidebar: [
				{
					label: 'Getting started',
					items: [
						'getting-started/introduction',
						'getting-started/quick-start',
						'getting-started/installation',
					],
				},
				{
					label: 'Integrate',
					items: [
						'integrate/overview',
						'integrate/proxy-mode',
						'integrate/gateway-mode',
						'integrate/rust',
						'integrate/docker-kubernetes',
						'integrate/any-endpoint',
					],
				},
				{
					label: 'Concepts',
					items: [
						'concepts/how-it-works',
						'concepts/personas',
						'concepts/vault',
						'concepts/detection',
						'concepts/intent-policy',
						'concepts/file-masking',
						'concepts/tool-calls',
					],
				},
				{ label: 'Benchmarks', collapsed: true, items: [{ autogenerate: { directory: 'benchmarks' } }] },
				{ label: 'Compare', collapsed: true, items: [{ autogenerate: { directory: 'compare' } }] },
				{ label: 'Use cases', collapsed: true, items: [{ autogenerate: { directory: 'use-cases' } }] },
				{ label: 'Industries', collapsed: true, items: [{ autogenerate: { directory: 'industries' } }] },
				{ label: 'Company size', collapsed: true, items: [{ autogenerate: { directory: 'company-size' } }] },
				{
					label: 'Reference',
					items: [
						'reference/configuration',
						'reference/cli',
						'reference/providers',
						'reference/security',
						'reference/faq',
						'reference/troubleshooting',
					],
				},
				{
					label: 'Project',
					items: ['project/roadmap', 'project/contributing', 'project/changelog'],
				},
			],
		}),
	],
});
