// One Open Graph card per top-level docs section, committed as public/og/<dir>.png.
// Regenerate after editing this list:  node scripts/og-images.mjs
// The middleware (src/seo/middleware.ts) uses `dir` to pick the image and `alt` for
// og:image:alt / twitter:image:alt. Pages outside these sections use public/og.png.

/** @type {{ dir: string; label: string; headline: string; alt: string }[]} */
export const OG_CARDS = [
	{
		dir: 'getting-started',
		label: 'Getting started',
		headline: 'Mask PII in LLM prompts in five minutes',
		alt: 'install Shadow and mask PII in LLM prompts in five minutes',
	},
	{
		dir: 'integrate',
		label: 'Integrate',
		headline: 'Proxy, gateway, sidecar or Rust library',
		alt: 'run Shadow as a local HTTPS proxy, a gateway base URL, a container sidecar or a Rust library',
	},
	{
		dir: 'concepts',
		label: 'Concepts',
		headline: 'Detect, replace with fakes, restore',
		alt: 'how Shadow detects sensitive values, replaces them with reversible fakes and restores them',
	},
	{
		dir: 'benchmarks',
		label: 'Benchmarks',
		headline: 'Reproducible PII redaction benchmark',
		alt: 'leak rate, over-masking and round trip for Shadow and open-source PII tools',
	},
	{
		dir: 'compare',
		label: 'Compare',
		headline: 'Shadow vs PII redaction and AI DLP tools',
		alt: 'how Shadow compares with PII redaction, AI DLP and AI gateway tools',
	},
	{
		dir: 'use-cases',
		label: 'Use cases',
		headline: 'Keep customer data and secrets out of AI',
		alt: 'use cases for masking PII and secrets in prompts to AI APIs',
	},
	{
		dir: 'industries',
		label: 'Industries',
		headline: 'Use LLMs with regulated data',
		alt: 'using LLMs with regulated data in healthcare, finance, legal and other industries',
	},
	{
		dir: 'company-size',
		label: 'Company size',
		headline: 'From one laptop to a managed fleet',
		alt: 'rolling out Shadow from a single laptop to a managed fleet',
	},
	{
		dir: 'reference',
		label: 'Reference',
		headline: 'CLI, configuration, providers, security',
		alt: 'reference for the Shadow CLI, configuration, providers and security model',
	},
	{
		dir: 'project',
		label: 'Project',
		headline: 'Open source under Apache-2.0',
		alt: 'the Shadow roadmap, changelog and contribution guide',
	},
];
