// Single source of truth for where the docs are published and how the project is described.
// Override at build time to move the site, for example to a custom domain:
//
//   SITE=https://shadow.example.com BASE=/ pnpm build
//
// Defaults publish to GitHub Pages at https://tiny-bubble-company.github.io/shadow/.

const rawSite = process.env.SITE || 'https://tiny-bubble-company.github.io';
const rawBase = process.env.BASE ?? '/shadow';

/** Origin (and optional path) of the deployment, without a trailing slash. */
export const SITE = rawSite.replace(/\/+$/, '');

/** Base path: '' for a site served at the domain root, otherwise '/name' (no trailing slash). */
export const BASE = rawBase.replace(/^\/*/, '/').replace(/\/+$/, '') === '/'
	? ''
	: rawBase.replace(/^\/*/, '/').replace(/\/+$/, '');

/** Absolute URL of the docs home, with a trailing slash. */
export const HOME_URL = `${SITE}${BASE}/`;

export const REPO_URL = 'https://github.com/Tiny-Bubble-Company/shadow';
export const RELEASES_URL = `${REPO_URL}/releases`;
export const ORG_NAME = 'Tiny Bubble Company';
export const ORG_URL = 'https://github.com/Tiny-Bubble-Company';

export const SITE_NAME = 'Shadow';
export const TAGLINE = 'open-source privacy layer for AI';
export const SITE_DESCRIPTION =
	'Shadow is an open-source privacy layer for AI: it swaps names, emails, customer data and secrets for realistic, reversible fakes before prompts reach ChatGPT, Claude or any LLM, then restores them in the answer.';
