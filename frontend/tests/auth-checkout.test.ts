import { cleanup, render, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { goto } from '$app/navigation';
import AuthForm from '../src/lib/components/AuthForm.svelte';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(() => Promise.resolve()) }));

const calls: { url: string; body?: string }[] = [];
const fetchMock = vi.fn(async (url: string, init: RequestInit = {}) => {
  calls.push({ url, body: init.body as string | undefined });
  const json = url.endsWith('/billing')
    ? { enabled: true, checkoutAvailable: true, plans: [{ id: 'pro' }] }
    : { url: 'https://checkout.stripe.test/session', portal: false };
  return new Response(JSON.stringify(json), { status: 200, headers: { 'content-type': 'application/json' } });
});
const values = new Map<string, string>([['slimlytics_token', 'session-token']]);
const assign = vi.fn();

describe('auth form plan continuation', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', fetchMock);
    vi.stubGlobal('localStorage', { getItem: (key: string) => values.get(key) ?? null, setItem: () => {}, removeItem: () => {} });
  });
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.clearAllMocks();
    calls.length = 0;
  });

  const at = (search: string) =>
    vi.stubGlobal('location', { search, origin: 'http://localhost', href: `http://localhost/register${search}`, assign });

  it('continues a paid pricing link straight to Stripe Checkout', async () => {
    at('?plan=pro&interval=year');
    render(AuthForm, { mode: 'register' });
    await waitFor(() => expect(assign).toHaveBeenCalledWith('https://checkout.stripe.test/session'));
    const checkout = calls.find((call) => call.url.endsWith('/billing/checkout'))!;
    expect(JSON.parse(checkout.body!)).toEqual({ plan: 'pro', interval: 'year' });
  });

  it('goes to the dashboard without a paid plan', async () => {
    at('?plan=free');
    render(AuthForm, { mode: 'register' });
    await waitFor(() => expect(goto).toHaveBeenCalledWith('/app'));
    expect(calls.some((call) => call.url.includes('/billing'))).toBe(false);
  });
});
