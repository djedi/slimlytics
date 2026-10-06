import { cleanup, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import MarketingHeader from '../src/lib/components/marketing/MarketingHeader.svelte';

const values = new Map<string, string>();
vi.stubGlobal('localStorage', {
  getItem: (key: string) => values.get(key) ?? null,
  setItem: (key: string, value: string) => values.set(key, value),
  removeItem: (key: string) => values.delete(key),
  clear: () => values.clear()
});

describe('marketing header', () => {
  beforeEach(() => {
    cleanup();
    values.clear();
  });

  it('links to pricing, privacy, docs, and signup when logged out', () => {
    render(MarketingHeader);
    expect(screen.getByRole('link', { name: 'Slimlytics on GitHub' })).toHaveAttribute('href', 'https://github.com/djedi/slimlytics');
    expect(screen.getByRole('link', { name: 'Pricing' })).toHaveAttribute('href', '/pricing');
    expect(screen.getByRole('link', { name: 'Privacy' })).toHaveAttribute('href', '/privacy');
    expect(screen.getByRole('link', { name: 'Docs' })).toHaveAttribute('href', '/docs');
    expect(screen.getByRole('link', { name: 'Sign in' })).toHaveAttribute('href', '/login');
    expect(screen.getByRole('link', { name: 'Get started' })).toHaveAttribute('href', '/register');
  });

  it('server-renders a hidden dashboard link for the inline auth script to reveal', () => {
    render(MarketingHeader);
    const dashboard = document.querySelector('a[data-auth="in"]');
    expect(dashboard).toHaveTextContent('Open dashboard');
    expect(dashboard).toHaveAttribute('href', '/app');
    expect(dashboard).toHaveAttribute('data-auth', 'in');
    expect(dashboard).not.toBeVisible();
    expect(screen.getByRole('link', { name: 'Sign in' })).toHaveAttribute('data-auth', 'out');
  });

  it('exposes an accessible mobile menu toggle wired to the navigation', () => {
    render(MarketingHeader);
    const toggle = screen.getByRole('button', { name: 'Open menu' });
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
    expect(toggle).toHaveAttribute('aria-controls', 'mkt-nav');
  });

  it('offers light, dark, and system theme radios, defaulting to system', () => {
    render(MarketingHeader);
    const group = screen.getByRole('group', { name: 'Color theme' });
    expect(group).toBeInTheDocument();
    for (const name of ['Light', 'Dark', 'System']) {
      expect(screen.getByRole('radio', { name })).toHaveAttribute('name', 'mkt-theme');
    }
    expect(screen.getByRole('radio', { name: 'System' })).toBeChecked();
  });
});
