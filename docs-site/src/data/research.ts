// Published research articles (src/content/research), newest first, plus topic helpers.
// Used by the research index and article pages, the home page section and the SEO middleware.
import { getCollection, type CollectionEntry } from 'astro:content';
import { PRIMARY_TOPICS } from './research-topics';
import { sitePath } from '../nav/site-nav';

export type ResearchEntry = CollectionEntry<'research'>;

let cache: Promise<ResearchEntry[]> | undefined;
export function getResearch(): Promise<ResearchEntry[]> {
	cache ??= getCollection('research', (e) => !e.data.draft).then((list) =>
		list.sort((a, b) => b.data.publishedDate.valueOf() - a.data.publishedDate.valueOf()),
	);
	return cache;
}

export const researchPath = (id: string) => sitePath(`/research/${id}/`);

/** Topic label -> anchor on the research index ('/research/#ai-coding-tools'). */
export function topicHref(label: string): string {
	const slug = PRIMARY_TOPICS.find((t) => t.label === label)?.slug;
	return slug ? `${sitePath('/research/')}#${slug}` : sitePath('/research/');
}

/** Explicit `related` slugs first, then the articles sharing the most topics. */
export async function relatedResearch(entry: ResearchEntry, limit = 3): Promise<ResearchEntry[]> {
	const all = (await getResearch()).filter((e) => e.id !== entry.id);
	const picked = entry.data.related.map((id) => all.find((e) => e.id === id)).filter((e): e is ResearchEntry => !!e);
	const score = (e: ResearchEntry) => e.data.topics.filter((t) => entry.data.topics.includes(t)).length;
	const rest = all
		.filter((e) => !picked.includes(e))
		.map((e) => ({ e, s: score(e) }))
		.sort((a, b) => b.s - a.s)
		.map(({ e }) => e);
	return [...picked, ...rest].slice(0, limit);
}

export const formatDate = (date: Date, month: 'long' | 'short' = 'long') =>
	date.toLocaleDateString('en-US', { year: 'numeric', month, day: 'numeric', timeZone: 'UTC' });
