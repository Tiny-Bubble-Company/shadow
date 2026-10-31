// Links used by the home page components. Paths are written without the base
// ('/getting-started/quick-start/') and get the configured BASE prepended, so the same
// markup works at the domain root and under /shadow on GitHub Pages.
import { REPO_URL } from '../../site.mjs';

const BASE = (import.meta.env.BASE_URL ?? '/').replace(/\/+$/, '');

/** Prefix a site-internal path with the configured base. External URLs pass through. */
export function href(path: string): string {
	if (/^(https?:)?\/\//.test(path) || path.startsWith('#')) return path;
	return `${BASE}${path.startsWith('/') ? path : `/${path}`}`;
}

export const LINKS = {
	quickStart: href('/getting-started/quick-start/'),
	installation: href('/getting-started/installation/'),
	introduction: href('/getting-started/introduction/'),
	howItWorks: href('/concepts/how-it-works/'),
	intent: href('/concepts/intent-policy/'),
	benchmarks: href('/benchmarks/'),
	compare: href('/compare/'),
	useCases: href('/use-cases/'),
	industries: href('/industries/'),
	companySize: href('/company-size/'),
	integrate: href('/integrate/overview/'),
	providers: href('/reference/providers/'),
	security: href('/reference/security/'),
	faq: href('/reference/faq/'),
	cli: href('/reference/cli/'),
	repo: REPO_URL,
} as const;
