// Generates one 1200×630 Open Graph card per docs section into public/og/<dir>.png,
// from the list in src/seo/og-cards.mjs. The PNGs are committed, so the build needs
// nothing extra; run this again after changing the list or the design:
//
//   node scripts/og-images.mjs

import { mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import sharp from 'sharp';
import { OG_CARDS } from '../src/seo/og-cards.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const outDir = join(root, 'public', 'og');
mkdirSync(outDir, { recursive: true });

const esc = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

/** Greedy word wrap for the headline (approximate glyph widths for a bold sans). */
function wrap(text, maxChars) {
	const lines = [];
	let line = '';
	for (const word of text.split(' ')) {
		if ((line + ' ' + word).trim().length > maxChars && line) {
			lines.push(line);
			line = word;
		} else line = (line + ' ' + word).trim();
	}
	if (line) lines.push(line);
	return lines;
}

const FONT = "'Inter', 'Helvetica Neue', Helvetica, Arial, sans-serif";
const MONO = "'JetBrains Mono', Menlo, 'DejaVu Sans Mono', monospace";

function card({ label, headline }) {
	const lines = wrap(headline, 28);
	const size = lines.length > 2 ? 64 : 76;
	const lineHeight = size * 1.12;
	const top = 330 - ((lines.length - 1) * lineHeight) / 2;
	return `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <defs>
    <radialGradient id="bg" cx="0.28" cy="0.4" r="0.9">
      <stop offset="0" stop-color="#2a2160"/>
      <stop offset="0.55" stop-color="#151230"/>
      <stop offset="1" stop-color="#0c0b1d"/>
    </radialGradient>
    <linearGradient id="g" x1="10" y1="8" x2="54" y2="58" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#C4B8FF"/>
      <stop offset="0.55" stop-color="#7C6CFF"/>
      <stop offset="1" stop-color="#4F3FE0"/>
    </linearGradient>
    <linearGradient id="text" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#ffffff"/>
      <stop offset="1" stop-color="#d9d2ff"/>
    </linearGradient>
    <mask id="m">
      <rect width="64" height="64" fill="#fff"/>
      <circle cx="41" cy="25" r="19" fill="#000"/>
    </mask>
    <pattern id="grid" width="48" height="48" patternUnits="userSpaceOnUse">
      <path d="M48 0H0V48" fill="none" stroke="#ffffff" stroke-opacity="0.035"/>
    </pattern>
  </defs>
  <rect width="1200" height="630" fill="url(#bg)"/>
  <rect width="1200" height="630" fill="url(#grid)"/>
  <g transform="translate(72 64) scale(1.25)">
    <circle cx="32" cy="32" r="27" fill="none" stroke="url(#g)" stroke-width="2" stroke-opacity="0.45"/>
    <circle cx="32" cy="32" r="22" fill="url(#g)" mask="url(#m)"/>
  </g>
  <text x="168" y="122" font-family="${FONT}" font-size="44" font-weight="700" fill="#ffffff">Shadow</text>
  <rect x="72" y="196" rx="22" ry="22" width="${label.length * 15 + 56}" height="44" fill="#7C6CFF" fill-opacity="0.16" stroke="#7C6CFF" stroke-opacity="0.55"/>
  <text x="100" y="226" font-family="${FONT}" font-size="24" font-weight="600" fill="#c9c0ff">${esc(label)}</text>
  ${lines
		.map(
			(l, i) =>
				`<text x="72" y="${Math.round(top + 48 + i * lineHeight)}" font-family="${FONT}" font-size="${size}" font-weight="800" fill="url(#text)">${esc(l)}</text>`,
		)
		.join('\n  ')}
  <text x="72" y="566" font-family="${MONO}" font-size="24" fill="#a59cd9">jane@acme.com  →  hui.zhao@zhilan.cn  →  jane@acme.com</text>
  <text x="1128" y="566" text-anchor="end" font-family="${FONT}" font-size="24" font-weight="600" fill="#8f86c9">getshadow.dev</text>
</svg>`;
}

for (const c of OG_CARDS) {
	const file = join(outDir, `${c.dir}.png`);
	await sharp(Buffer.from(card(c))).png({ compressionLevel: 9, palette: true, quality: 90 }).toFile(file);
	console.log(`wrote public/og/${c.dir}.png`);
}
