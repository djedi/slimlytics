import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const sitemap = readFileSync(resolve(__dirname, '../static/sitemap.xml'), 'utf8');
const robots = readFileSync(resolve(__dirname, '../static/robots.txt'), 'utf8');
const locs = [...sitemap.matchAll(/<loc>([^<]+)<\/loc>/g)].map((match) => match[1]);

describe('sitemap.xml', () => {
  it('lists the public marketing and docs pages', () => {
    for (const path of ['/', '/pricing', '/privacy', '/about', '/docs', '/docs/mcp', '/docs/cli', '/docs/api']) {
      expect(locs).toContain(`https://slimlytics.com${path}`);
    }
  });

  it('never lists pages robots.txt disallows', () => {
    const disallowed = [...robots.matchAll(/^Disallow:\s*(\S+)/gm)].map((match) => match[1]);
    for (const loc of locs) {
      const path = new URL(loc).pathname;
      expect(disallowed.some((prefix) => path.startsWith(prefix))).toBe(false);
    }
  });

  it('is referenced from robots.txt', () => {
    expect(robots).toMatch(/^Sitemap: https:\/\/slimlytics\.com\/sitemap\.xml$/m);
  });
});
