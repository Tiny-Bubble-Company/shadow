// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

const repo = 'https://github.com/Tiny-Bubble-Company/shadow';

// https://astro.build/config
export default defineConfig({
	// Published to GitHub Pages: https://tiny-bubble-company.github.io/shadow
	site: 'https://tiny-bubble-company.github.io',
	base: '/shadow',
	integrations: [
		starlight({
			title: 'Shadow',
			description:
				'Privacy layer for AI traffic. Shadow masks sensitive values with realistic, reversible fakes before requests leave your machine, and restores them in the response.',
			logo: { src: './src/assets/logo.svg', alt: 'Shadow' },
			favicon: '/favicon.svg',
			social: [{ icon: 'github', label: 'GitHub', href: repo }],
			editLink: { baseUrl: `${repo}/edit/main/docs-site/` },
			lastUpdated: false,
			customCss: ['./src/styles/custom.css'],
			head: [
				{ tag: 'meta', attrs: { name: 'theme-color', content: '#7C6CFF' } },
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
