// Last-modified / first-published dates for content files, from git history.
// Falls back to the build time for files that are not committed yet, or when git or
// history is unavailable (shallow CI clones should use fetch-depth: 0).

import { execFileSync } from 'node:child_process';

const BUILD_TIME = new Date().toISOString();
const cache = new Map<string, { modified: string; published: string }>();

function git(args: string[]): string {
	try {
		return execFileSync('git', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();
	} catch {
		return '';
	}
}

export function fileDates(filePath: string | undefined): { modified: string; published: string } {
	if (!filePath) return { modified: BUILD_TIME, published: BUILD_TIME };
	const hit = cache.get(filePath);
	if (hit) return hit;
	const log = git(['log', '--follow', '--format=%cI', '--', filePath]).split('\n').filter(Boolean);
	// Uncommitted edits make the working copy newer than the last commit.
	const dirty = git(['status', '--porcelain', '--', filePath]) !== '';
	const modified = dirty || log.length === 0 ? BUILD_TIME : log[0]!;
	const published = log.length ? log[log.length - 1]! : BUILD_TIME;
	const out = { modified, published };
	cache.set(filePath, out);
	return out;
}
