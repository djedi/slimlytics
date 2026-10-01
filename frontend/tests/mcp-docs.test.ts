import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import McpDocs from '../src/lib/components/McpDocs.svelte';
import DocsHub from '../src/routes/docs/+page.svelte';

describe('public MCP documentation', () => {
  it('links the MCP guide from the documentation hub', () => {
    render(DocsHub);
    expect(screen.getByRole('link', { name: 'Open MCP setup guide' })).toHaveAttribute('href', '/docs/mcp');
  });

  it('documents browser login, website installation, verification, and connection expiry', () => {
    render(McpDocs);
    expect(screen.getByRole('heading', { name: 'Set up analytics with your agent' })).toBeInTheDocument();
    expect(document.body).toHaveTextContent('codex mcp add slimlytics --url https://slimlytics.com/api/mcp');
    expect(document.body).toHaveTextContent('codex mcp login slimlytics');
    expect(document.body).toHaveTextContent('setup_site');
    expect(document.body).toHaveTextContent('tracking_setup');
    expect(document.body).toHaveTextContent('scriptTestUrl');
    expect(document.body).toHaveTextContent('beaconTestUrl');
    expect(document.body).toHaveTextContent(/30 days/);
    expect(document.body).toHaveTextContent(/consent, DNT, and GPC/);
    expect(screen.getByRole('link', { name: 'Documentation overview' })).toHaveAttribute('href', '/docs');
  });
});
