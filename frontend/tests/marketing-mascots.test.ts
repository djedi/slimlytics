import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';

const homepage = readFileSync('src/routes/(marketing)/+page.svelte', 'utf8');
const provenance = JSON.parse(readFileSync('static/images/mascot/codex-poses-provenance.json', 'utf8'));

describe('homepage mascot artwork', () => {
  it('uses the three real Codex-generated poses with reserved, lazy decorative images', () => {
    for (const name of ['lookout', 'helper', 'team']) {
      const image = homepage.match(new RegExp(`<img[^>]+src="/images/mascot/meerkat-${name}\\.webp"[^>]*>`))?.[0];
      expect(image, name).toBeDefined();
      expect(image).toContain('alt=""');
      expect(image).toContain('loading="lazy"');
      expect(image).toContain('decoding="async"');
      const asset = provenance.assets.find((item: { web_asset: string }) => item.web_asset.endsWith(`meerkat-${name}.webp`));
      expect(image).toContain(`width="${asset.width}"`);
      expect(image).toContain(`height="${asset.height}"`);
      const bytes = readFileSync(`static${asset.web_asset}`);
      expect(bytes.subarray(8, 12).toString()).toBe('WEBP');
      expect(bytes.length).toBeLessThan(100_000);
      expect(asset.alpha_extrema).toEqual([0, 255]);
    }
  });
  it('retains the existing hero, tour, privacy and reports artwork', () => {
    expect(homepage).toContain('meerkat-wave.webp');
    expect(homepage).toContain('meerkat-watch.webp');
    expect(homepage).toContain("'shield' : 'reports'");
  });
});
