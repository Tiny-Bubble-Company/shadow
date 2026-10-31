// Tiny inline-Markdown renderer for frontmatter strings (FAQ answers). Supports links,
// `code`, **bold** and *italic*. Everything else is escaped.

const BASE = (import.meta.env.BASE_URL ?? '/').replace(/\/+$/, '');
const AUTHORED_BASE = '/shadow';

function escapeHtml(s: string): string {
	return s
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;');
}

/** Rewrite links authored for the default `/shadow` base to the configured base. */
export function withBase(href: string): string {
	if (href === AUTHORED_BASE || href.startsWith(`${AUTHORED_BASE}/`)) {
		return (BASE + href.slice(AUTHORED_BASE.length)) || '/';
	}
	return href;
}

export function inlineMarkdownToHtml(md: string): string {
	const codes: string[] = [];
	let s = md.replace(/`([^`]+)`/g, (_, c) => {
		codes.push(`<code>${escapeHtml(c)}</code>`);
		return `\u0000${codes.length - 1}\u0000`;
	});
	s = escapeHtml(s);
	s = s.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, text, href) => {
		const url = withBase(href.replace(/&amp;/g, '&'));
		const external = /^https?:\/\//.test(url);
		return `<a href="${escapeHtml(url)}"${external ? ' rel="noopener"' : ''}>${text}</a>`;
	});
	s = s.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
	s = s.replace(/(^|[^*])\*([^*\s][^*]*)\*/g, '$1<em>$2</em>');
	s = s.replace(/\u0000(\d+)\u0000/g, (_, i) => codes[Number(i)] ?? '');
	return s;
}

/** Plain text version for JSON-LD and meta tags. */
export function inlineMarkdownToText(md: string): string {
	return md
		.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
		.replace(/`([^`]+)`/g, '$1')
		.replace(/\*\*([^*]+)\*\*/g, '$1')
		.replace(/(^|[^*])\*([^*\s][^*]*)\*/g, '$1$2')
		.trim();
}
