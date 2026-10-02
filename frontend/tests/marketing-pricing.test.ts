import { cleanup, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { pricingPlans } from '../src/lib/marketing/pricing';
import Page from '../src/routes/(marketing)/pricing/+page.svelte';

const values = new Map<string, string>();
vi.stubGlobal('localStorage', {
  getItem: (key: string) => values.get(key) ?? null,
  setItem: (key: string, value: string) => values.set(key, value),
  removeItem: (key: string) => values.delete(key),
  clear: () => values.clear()
});

describe('marketing pricing page', () => {
  beforeEach(() => {
    cleanup();
    values.clear();
  });

  it('renders all plans with signup CTAs', () => {
    render(Page);

    for (const plan of pricingPlans) {
      expect(screen.getAllByText(plan.name).length).toBeGreaterThan(0);
      expect(screen.getAllByText(plan.price).length).toBeGreaterThan(0);
    }

    const accountLinks = screen.getAllByRole('link', { name: /create account|get started free|start (pro|business)/i });
    expect(accountLinks.length).toBeGreaterThanOrEqual(3);
    expect(accountLinks.some((link) => link.getAttribute('href')?.startsWith('/register'))).toBe(
      true
    );
  });

  it('links paid plans to sign-up with the plan preselected', () => {
    render(Page);
    expect(screen.getByRole('link', { name: /start pro/i }).getAttribute('href')).toBe('/register?plan=pro');
  });

  it('shows a feature comparison table', () => {
    render(Page);
    expect(screen.getByRole('table')).toBeInTheDocument();
    expect(screen.getByText(/cookieless, privacy-first tracking/i)).toBeInTheDocument();
  });

  it('undercuts Clicky with daily limits and every feature on every plan', () => {
    render(Page);
    const prices = Object.fromEntries(pricingPlans.map((plan) => [plan.id, plan.price]));
    expect(prices).toEqual({ 'self-hosted': '$0', free: '$0', pro: '$7', business: '$15' });
    expect(screen.getAllByText('3,000 page views / day').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Every feature included').length).toBe(pricingPlans.length);
    expect(screen.getByText('or $56 / year — save 33%')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Built in Rust' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'MCP server for AI agents' })).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: 'Business' })).toBeInTheDocument();
  });
});
