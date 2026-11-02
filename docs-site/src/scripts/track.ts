// First-party funnel events: POST /api/e with navigator.sendBeacon (see worker/README.md).
// No cookies, no identifiers. Only the page path is sent, never its query string or
// fragment, so a workspace token in a link can never reach analytics.

export type Props = Record<string, string>;

export function track(event: string, props: Props = {}): void {
	const body = JSON.stringify({ event, path: location.pathname, props });
	try {
		navigator.sendBeacon('/api/e', new Blob([body], { type: 'application/json' }));
	} catch {
		/* analytics must never break the page */
	}
}

function propsOf(el: HTMLElement): Props {
	try {
		return JSON.parse(el.dataset.trackProps || '{}') as Props;
	} catch {
		return {};
	}
}

// Declarative: <a data-track="compare_cta" data-track-props='{"competitor":"nightfall"}'>.
document.addEventListener('click', (e) => {
	const el = (e.target as Element | null)?.closest<HTMLElement>('[data-track]');
	if (el?.dataset.track) track(el.dataset.track, propsOf(el));
});

// Comparison pages: any link to Hub, pricing or the Core quick start, or any form button,
// counts as compare_cta for that competitor (the page slug).
const compareSlug = /\/compare\/([^/]+)\/?$/.exec(location.pathname)?.[1];
if (compareSlug) {
	document.addEventListener('click', (e) => {
		const el = (e.target as Element | null)?.closest<HTMLElement>('a[href], [data-open-form], [data-copy-cmd]');
		if (!el) return;
		const href = el.getAttribute('href') ?? '';
		if (el.matches('[data-open-form], [data-copy-cmd]') || /\/(hub|pricing|getting-started\/quick-start)\//.test(href)) {
			track('compare_cta', { competitor: compareSlug });
		}
	});
}

// Copy buttons: <button data-copy-cmd="..." data-track="install_copy" ...>.
document.addEventListener('click', async (e) => {
	const btn = (e.target as Element | null)?.closest<HTMLElement>('[data-copy-cmd]');
	if (!btn) return;
	const label = btn.querySelector('[data-copy-label]');
	const original = label?.textContent;
	try {
		await navigator.clipboard.writeText(btn.dataset.copyCmd ?? '');
		if (label) label.textContent = 'Copied';
	} catch {
		if (label) label.textContent = 'Select and copy';
	}
	setTimeout(() => label && original && (label.textContent = original), 2000);
});
