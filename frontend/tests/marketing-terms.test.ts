import { cleanup, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import TermsPage from '../src/routes/(marketing)/terms/+page.svelte';
import PrivacyPage from '../src/routes/(marketing)/privacy/+page.svelte';
import MarketingFooter from '../src/lib/components/marketing/MarketingFooter.svelte';

describe('terms of service and data processing note', () => {
  beforeEach(() => cleanup());

  it('renders the terms with billing, data, and acceptable-use sections', () => {
    render(TermsPage);
    expect(screen.getByRole('heading', { level: 1, name: 'Terms of Service' })).toBeInTheDocument();
    for (const name of [/plans, limits, and billing/i, /your data and your visitors/i, /acceptable use/i, /limitation of liability/i]) {
      expect(screen.getByRole('heading', { name })).toBeInTheDocument();
    }
    expect(screen.getByRole('link', { name: /data processing and hosting/i })).toHaveAttribute('href', '/privacy#data-processing');
    expect(screen.getByText(/laws of the State of Utah/)).toBeInTheDocument();
    expect(screen.getByText(/end of the current billing\s+period/)).toBeInTheDocument();
  });

  it('adds a data processing and hosting section to the privacy page', () => {
    const { container } = render(PrivacyPage);
    expect(container.querySelector('#data-processing')).not.toBeNull();
    expect(screen.getByRole('heading', { name: /data processing and hosting/i })).toBeInTheDocument();
    expect(screen.getByText(/Stripe: payment processing/)).toBeInTheDocument();
    expect(container.textContent).toMatch(/HostRush \(ServerCheap\)/);
    expect(container.textContent).toMatch(/signed data processing agreement/);
  });

  it('links the terms from the marketing footer', () => {
    render(MarketingFooter);
    expect(screen.getByRole('link', { name: 'Terms' })).toHaveAttribute('href', '/terms');
  });
});
