import { cleanup, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import AboutPage from '../src/routes/(marketing)/about/+page.svelte';
import MarketingFooter from '../src/lib/components/marketing/MarketingFooter.svelte';

describe('marketing about page', () => {
  beforeEach(() => cleanup());

  it('says who builds Slimlytics and explains the AGPL', () => {
    render(AboutPage);
    expect(screen.getByRole('heading', { level: 1 })).toBeInTheDocument();
    expect(screen.getAllByText(/Dustin Davis/).length).toBeGreaterThan(0);
    expect(screen.getByRole('heading', { name: /AGPL-3\.0/ })).toBeInTheDocument();
    expect(screen.getAllByRole('link', { name: /github/i })[0]).toHaveAttribute('href', 'https://github.com/djedi/slimlytics');
  });

  it('has a contact section with issue and private security channels', () => {
    const { container } = render(AboutPage);
    expect(container.querySelector('#contact')).not.toBeNull();
    expect(screen.getByRole('link', { name: /open an issue/i })).toHaveAttribute('href', 'https://github.com/djedi/slimlytics/issues');
    expect(screen.getByRole('link', { name: /security policy/i })).toHaveAttribute('href', 'https://github.com/djedi/slimlytics/blob/main/SECURITY.md');
    expect(container.textContent).not.toMatch(/TODO/);
  });

  it('is linked from the marketing footer', () => {
    render(MarketingFooter);
    expect(screen.getByRole('link', { name: 'About' })).toHaveAttribute('href', '/about');
    expect(screen.getByRole('link', { name: 'Contact' })).toHaveAttribute('href', '/about#contact');
  });
});
