import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import McpDocs from '../src/lib/components/McpDocs.svelte';
import DocsHub from '../src/routes/docs/+page.svelte';

describe('public MCP documentation', () => {
  afterEach(cleanup);

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

  it('documents Claude Code and Hermes Agent connections with example prompts', () => {
    render(McpDocs);
    expect(screen.getByRole('heading', { name: 'Claude Code' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Hermes Agent' })).toBeInTheDocument();
    expect(document.body).toHaveTextContent('claude mcp add --transport http slimlytics https://slimlytics.com/api/mcp');
    expect(document.body).toHaveTextContent('"type": "http"');
    expect(document.body).toHaveTextContent('mcp__slimlytics__analytics_summary');
    expect(document.body).toHaveTextContent('auth: oauth');
    expect(document.body).toHaveTextContent('hermes mcp login slimlytics');
    expect(screen.getByRole('heading', { name: 'Prompt library' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Copy .mcp.json' })).toBeInTheDocument();
  });

  it('limits the headless Claude Code example to read-only reporting tools', () => {
    render(McpDocs);
    const headless = [...document.querySelectorAll('pre code')]
      .map((node) => node.textContent ?? '')
      .find((code) => code.includes('claude -p'));
    expect(headless).toBeDefined();
    expect(headless).not.toMatch(/--allowedTools "mcp__slimlytics"/);
    expect(headless).toContain('mcp__slimlytics__analytics_summary');
    expect(headless).not.toContain('setup_site');
  });
});
