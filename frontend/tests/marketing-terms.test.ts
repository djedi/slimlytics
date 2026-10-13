import { cleanup, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import TermsPage from '../src/routes/(marketing)/terms/+page.svelte';
import PrivacyPage from '../src/routes/(marketing)/privacy/+page.svelte';
import MarketingFooter from '../src/lib/components/marketing/MarketingFooter.svelte';

describe('terms of service', () => {
  beforeEach(() => cleanup());

  it('shows effective date, Search Console terms, and contact', () => {
    render(TermsPage);
    expect(screen.getByRole('heading', { level: 1, name: /terms of service/i })).toBeInTheDocument();
    expect(screen.getByText(/effective october 13, 2026/i)).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /google search console/i })).toBeInTheDocument();
    expect(screen.getAllByText(/webmasters\.readonly/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/limited use/i).length).toBeGreaterThan(0);
    expect(screen.getAllByRole('link', { name: /dustin@redseam\.com/i })[0]).toHaveAttribute(
      'href',
      'mailto:dustin@redseam.com'
    );
  });

  it('privacy page includes the Google API Limited Use disclosure', () => {
    render(PrivacyPage);
    expect(screen.getByRole('heading', { name: /google api services/i })).toBeInTheDocument();
    expect(screen.getAllByText(/limited use/i).length).toBeGreaterThan(0);
  });

  it('footer links to terms next to privacy', () => {
    render(MarketingFooter);
    expect(screen.getByRole('link', { name: 'Terms' })).toHaveAttribute('href', '/terms');
  });
});
