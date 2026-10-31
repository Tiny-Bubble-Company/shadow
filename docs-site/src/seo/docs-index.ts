// Shared helpers for the machine-readable endpoints (/llms.txt, /llms-full.txt).

import { getCollection, type CollectionEntry } from 'astro:content';
import { HOME_URL } from '../site.mjs';
import { SECTIONS, idToSlug, sectionLabel } from './sections';

export type DocEntry = CollectionEntry<'docs'>;

export const pageUrl = (entry: DocEntry) => {
	const slug = idToSlug(entry.id);
	return new URL(slug ? `${slug}/` : '', HOME_URL).href;
};

type Data = DocEntry['data'] & { sidebar?: { order?: number } };

/** All published pages grouped by section, in sidebar order. Home page excluded. */
export async function groupedPages() {
	const all = (await getCollection('docs')).filter(
		(e) => !e.data.draft && e.id !== 'index' && e.id !== '404' && !e.id.endsWith('/404'),
	);
	const byDir = new Map<string, DocEntry[]>();
	for (const e of all) {
		// Section index pages have the section name as id ('compare', not 'compare/index').
		const dir = e.id.split('/')[0]!;
		if (!byDir.has(dir)) byDir.set(dir, []);
		byDir.get(dir)!.push(e);
	}
	const known = SECTIONS.map((s) => s.dir);
	const dirs = [
		...known.filter((d) => byDir.has(d)),
		...[...byDir.keys()].filter((d) => !known.includes(d)).sort(),
	];
	return dirs.map((dir) => {
		const section = SECTIONS.find((s) => s.dir === dir);
		const rank = (e: DocEntry) => {
			const name = idToSlug(e.id).slice(dir.length + 1);
			if (name === '') return -1e9;
			const i = section?.pages?.indexOf(name) ?? -1;
			if (i >= 0) return i;
			return (e.data as Data).sidebar?.order ?? 1e6;
		};
		const entries = byDir
			.get(dir)!
			.sort((a, b) => rank(a) - rank(b) || a.data.title.localeCompare(b.data.title));
		return {
			dir,
			label: dir ? sectionLabel(dir) : 'Pages',
			optional: section?.optional ?? false,
			entries,
		};
	});
}

/** Turn MDX/Markdown source into plain Markdown that reads well without rendering. */
export function toPlainMarkdown(body: string): string {
	// Leave fenced code blocks untouched; clean up only the prose between them.
	const parts = body.split(/(^[ \t]*(?:```|~~~)[^\n]*\n[\s\S]*?^[ \t]*(?:```|~~~)[ \t]*$)/m);
	const out = parts.map((part, i) => (i % 2 === 1 ? part : cleanProse(part))).join('');
	// Absolute links for docs pages.
	return out
		.replace(/(\]\(|href=")\/shadow\//g, `$1${HOME_URL}`)
		.replace(/\n{3,}/g, '\n\n')
		.trim();
}

function cleanProse(text: string): string {
	let s = text
		// MDX imports/exports
		.replace(/^(import|export)\s.*$/gm, '')
		// Starlight components with useful attributes
		.replace(
			/<LinkCard\b([^>]*?)\/>/gs,
			(_, attrs: string) => {
				const get = (k: string) => attrs.match(new RegExp(`${k}="([^"]*)"`))?.[1];
				const t = get('title');
				const h = get('href');
				const d = get('description');
				return t && h ? `- [${t}](${h})${d ? `: ${d}` : ''}` : '';
			},
		)
		.replace(/<Card\b[^>]*title="([^"]*)"[^>]*>/g, '\n**$1**\n')
		.replace(/<TabItem\b[^>]*label="([^"]*)"[^>]*>/g, '\n**$1**\n')
		.replace(/<Aside\b[^>]*title="([^"]*)"[^>]*>/g, '\n> **$1**\n')
		// Any remaining capitalised (component) tags
		.replace(/<\/?[A-Z][\w.]*\b[^>]*?\/?>/g, '')
		// {'literal'} expressions used to escape text in MDX
		.replace(/\{'([^']*)'\}/g, '$1');
	// Rows of inline <span> cells (e.g. the landing page's before/after demo) become a table.
	s = s.replace(/(?:^[ \t]*(?:<span\b[^>]*>[^<]*<\/span>)+[ \t]*$\n?)+/gm, (block) => {
		const rows = block
			.trim()
			.split('\n')
			.map((line) => [...line.matchAll(/<span\b[^>]*>([^<]*)<\/span>/g)].map((m) => m[1]!.trim()));
		const width = Math.max(...rows.map((r) => r.length));
		const fmt = (r: string[]) => `| ${Array.from({ length: width }, (_, i) => r[i] ?? '').join(' | ')} |`;
		return [fmt(rows[0]!), fmt(rows[0]!.map(() => '---')), ...rows.slice(1).map(fmt)].join('\n') + '\n';
	});
	s = s
		.replace(/^[ \t]*<li>\s*/gm, '- ')
		.replace(/<strong>([^<]*)<\/strong>/g, '**$1**')
		// Remaining lowercase HTML tags (wrappers such as <div>, <ul>)
		.replace(/<\/?[a-z][\w-]*\b[^>]*?\/?>/g, '');
	return s;
}
