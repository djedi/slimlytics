import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SiteIcon from '../src/lib/components/SiteIcon.svelte';
import SiteIconSettings from '../src/lib/components/SiteIconSettings.svelte';
import { demoSites, type Site } from '../src/lib/api';

const site: Site = { ...demoSites[0], id: 'site-1', name: 'Northstar Docs' };

describe('site icon', () => {
  afterEach(cleanup);

  it('shows initials on the automatic hue by default', () => {
    const { container } = render(SiteIcon, { site });
    const icon = container.querySelector('.site-icon') as HTMLElement;
    expect(icon).toHaveTextContent('NO');
    expect(icon.style.background).toContain('linear-gradient');
  });

  it('uses custom colors for the tile and initials', () => {
    const { container } = render(SiteIcon, {
      site: { ...site, iconBackground: '#ff6600', iconBackgroundEnd: '#003366', iconForeground: '#000000' }
    });
    const icon = container.querySelector('.site-icon') as HTMLElement;
    expect(icon.style.background).toMatch(/linear-gradient\(140deg, (#ff6600|rgb\(255, 102, 0\)), (#003366|rgb\(0, 51, 102\))\)/);
    expect(icon.style.color).toMatch(/#000000|rgb\(0, 0, 0\)/);
  });

  it('shows the stored favicon, versioned, and falls back to initials if it fails', async () => {
    const { container } = render(SiteIcon, {
      site: { ...site, iconMode: 'favicon', iconUpdatedAt: '2026-10-06T00:00:00Z' }
    });
    const image = container.querySelector('img')!;
    expect(image).toHaveAttribute('src', '/api/sites/site-1/icon?v=2026-10-06T00%3A00%3A00Z');
    await fireEvent.error(image);
    expect(container.querySelector('img')).toBeNull();
    expect(container.querySelector('.site-icon')).toHaveTextContent('NO');
  });

  it('keeps initials in favicon mode until a favicon has been fetched', () => {
    const { container } = render(SiteIcon, { site: { ...site, iconMode: 'favicon', iconUpdatedAt: null } });
    expect(container.querySelector('img')).toBeNull();
  });
});

describe('site icon settings', () => {
  afterEach(cleanup);

  it('saves the chosen style and colors', async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const { container } = render(SiteIconSettings, { site, save });
    await fireEvent.click(screen.getByRole('radio', { name: /Website favicon/ }));
    const [background] = container.querySelectorAll<HTMLInputElement>('input[type=color]');
    await fireEvent.input(background, { target: { value: '#ff6600' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save icon' }));
    expect(save).toHaveBeenCalledWith({ mode: 'favicon', background: '#ff6600', backgroundEnd: '', foreground: '' });
    expect(await screen.findByRole('status')).toHaveTextContent('Icon saved.');
  });

  it('shows why a favicon could not be fetched', async () => {
    const save = vi.fn().mockRejectedValue(new Error("couldn't load a favicon for docs.northstar.dev"));
    render(SiteIconSettings, { site, save });
    await fireEvent.click(screen.getByRole('button', { name: 'Save icon' }));
    expect(await screen.findByRole('alert')).toHaveTextContent("couldn't load a favicon");
  });
});
