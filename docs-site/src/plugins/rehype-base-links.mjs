// Pages link to each other with root-relative URLs written for the default base, e.g.
// `/shadow/integrate/proxy-mode/`. When the site is built for a different base (a custom
// domain with BASE=/, say), rewrite those links so they keep working.

const AUTHORED_BASE = '/shadow';

/** @param {{ base: string }} opts base without trailing slash ('' for root) */
export default function rehypeBaseLinks({ base }) {
	if (base === AUTHORED_BASE) return () => {};
	const rewrite = (value) => {
		if (typeof value !== 'string') return value;
		if (value === AUTHORED_BASE) return base || '/';
		if (value.startsWith(`${AUTHORED_BASE}/`)) return base + value.slice(AUTHORED_BASE.length);
		return value;
	};
	const walk = (node) => {
		if (node.type === 'element' && node.properties) {
			for (const key of ['href', 'src']) {
				if (key in node.properties) node.properties[key] = rewrite(node.properties[key]);
			}
		}
		// MDX JSX elements (e.g. <LinkCard href="/shadow/..." />)
		if ((node.type === 'mdxJsxFlowElement' || node.type === 'mdxJsxTextElement') && node.attributes) {
			for (const attr of node.attributes) {
				if ((attr.name === 'href' || attr.name === 'link') && typeof attr.value === 'string') {
					attr.value = rewrite(attr.value);
				}
			}
		}
		if (node.children) node.children.forEach(walk);
	};
	return (tree) => walk(tree);
}
