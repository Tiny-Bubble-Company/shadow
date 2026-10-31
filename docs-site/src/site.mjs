// Single source of truth for where the site is published and how the project is described.
//
// SITE  origin of the deployment, no trailing slash. Default: https://getshadow.dev.
//       Used for canonical URLs, Open Graph, JSON-LD, the sitemap, robots.txt and llms.txt.
// BASE  path the site is served under. Default '' (the domain root).
//
//   pnpm dev                                         # http://localhost:4321/
//   pnpm build                                       # https://getshadow.dev/
//   SITE=https://docs.example.com pnpm build         # a self-hosted copy elsewhere

export const PROD_SITE = 'https://getshadow.dev';

const rawSite = process.env.SITE || PROD_SITE;
const rawBase = process.env.BASE ?? '';

/** Origin (and optional path) of the deployment, without a trailing slash. */
export const SITE = rawSite.replace(/\/+$/, '');

const normalisedBase = rawBase.replace(/^\/*/, '/').replace(/\/+$/, '');
/** Base path: '' for a site served at the domain root, otherwise '/name' (no trailing slash). */
export const BASE = normalisedBase === '/' ? '' : normalisedBase;

/** Absolute URL of the site home, with a trailing slash. */
export const HOME_URL = `${SITE}${BASE}/`;

export const REPO_URL = 'https://github.com/Tiny-Bubble-Company/shadow';
export const RELEASES_URL = `${REPO_URL}/releases`;
export const ORG_NAME = 'Tiny Bubble Company';
export const ORG_URL = 'https://github.com/Tiny-Bubble-Company';

export const SITE_NAME = 'Shadow';
export const TAGLINE = 'open-source privacy layer for AI';
export const SITE_DESCRIPTION =
	'Shadow is an open-source privacy layer for AI: it swaps names, emails, customer data and secrets for realistic, reversible fakes before prompts reach the APIs behind ChatGPT, Claude and other LLMs, then restores them in the answer.';
