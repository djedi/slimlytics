import { describe, expect, it, vi } from 'vitest';
import { ApiClient, type SessionStore } from '../src/lib/api';

function json(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
}

function memoryStore(token: string, refreshToken: string) {
  const state = { token, refreshToken };
  const store: SessionStore & { state: typeof state; cleared: boolean } = {
    state,
    cleared: false,
    load: () => ({ ...state }),
    save: (tokens) => Object.assign(state, tokens),
    clear() {
      state.token = '';
      state.refreshToken = '';
      this.cleared = true;
    }
  };
  return store;
}

describe('ApiClient sessions', () => {
  it('refreshes an expired access token once and retries the request', async () => {
    const fetcher = vi
      .fn()
      .mockResolvedValueOnce(json({ error: { message: 'expired' } }, 401))
      .mockResolvedValueOnce(json({ token: 'fresh', refreshToken: 'slrt_next', expiresIn: 3600 }))
      .mockResolvedValueOnce(json({ id: 'u1', email: 'a@example.com' }));
    const store = memoryStore('stale', 'slrt_first');
    const api = new ApiClient('/api', fetcher, false);
    api.useSession(store);
    const unauthorized = vi.fn();
    api.onUnauthorized = unauthorized;

    await expect(api.me()).resolves.toMatchObject({ id: 'u1' });

    expect(fetcher).toHaveBeenNthCalledWith(2, '/api/auth/refresh', expect.objectContaining({
      method: 'POST',
      body: JSON.stringify({ refreshToken: 'slrt_first' })
    }));
    expect(fetcher.mock.calls[2][1].headers.authorization).toBe('Bearer fresh');
    expect(store.state).toEqual({ token: 'fresh', refreshToken: 'slrt_next' });
    expect(api.accessToken).toBe('fresh');
    expect(unauthorized).not.toHaveBeenCalled();
  });

  it('shares one refresh between concurrent requests', async () => {
    let refreshes = 0;
    const fetcher = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url.endsWith('/auth/refresh')) {
        refreshes += 1;
        return json({ token: 'fresh', refreshToken: 'slrt_next', expiresIn: 3600 });
      }
      const auth = (init?.headers as Record<string, string>).authorization;
      return auth === 'Bearer fresh' ? json([]) : json({}, 401);
    });
    const api = new ApiClient('/api', fetcher, false);
    api.useSession(memoryStore('stale', 'slrt_first'));
    await Promise.all([api.sites(), api.sites(), api.sites()]);
    expect(refreshes).toBe(1);
  });

  it('adopts a token another tab already refreshed instead of spending the refresh token', async () => {
    const store = memoryStore('stale', 'slrt_first');
    const fetcher = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url.endsWith('/auth/refresh')) throw new Error('should not refresh');
      const auth = (init?.headers as Record<string, string>).authorization;
      if (auth === 'Bearer stale') {
        store.state.token = 'from-other-tab';
        store.state.refreshToken = 'slrt_other';
        return json({}, 401);
      }
      return json([]);
    });
    const api = new ApiClient('/api', fetcher, false);
    api.useSession(store);
    await expect(api.sites()).resolves.toEqual([]);
    expect(api.accessToken).toBe('from-other-tab');
  });

  it('signs out when the refresh token is rejected', async () => {
    const fetcher = vi
      .fn()
      .mockResolvedValueOnce(json({}, 401))
      .mockResolvedValueOnce(json({ error: { message: 'revoked' } }, 401));
    const store = memoryStore('stale', 'slrt_revoked');
    const api = new ApiClient('/api', fetcher, false);
    api.useSession(store);
    const unauthorized = vi.fn();
    api.onUnauthorized = unauthorized;
    await expect(api.me()).rejects.toMatchObject({ status: 401 });
    expect(unauthorized).toHaveBeenCalledOnce();
    expect(store.cleared).toBe(true);
    expect(fetcher).toHaveBeenCalledTimes(2);
  });

  it('never refreshes sign-in requests', async () => {
    const fetcher = vi.fn().mockResolvedValue(json({ error: { message: 'Bad password' } }, 401));
    const api = new ApiClient('/api', fetcher, false);
    api.useSession(memoryStore('t', 'slrt_x'));
    await expect(api.login('a@example.com', 'wrong')).rejects.toMatchObject({ status: 401 });
    expect(fetcher).toHaveBeenCalledTimes(1);
  });

  it('stores tokens from sign-in and revokes the session on sign-out', async () => {
    const fetcher = vi
      .fn()
      .mockResolvedValueOnce(json({ token: 'access', refreshToken: 'slrt_a', expiresIn: 3600 }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }));
    const store = memoryStore('', '');
    const api = new ApiClient('/api', fetcher, false);
    api.useSession(store);
    await api.login('a@example.com', 'correct horse battery staple');
    expect(store.state).toEqual({ token: 'access', refreshToken: 'slrt_a' });
    await api.logout();
    expect(fetcher).toHaveBeenLastCalledWith('/api/auth/logout', expect.objectContaining({ method: 'POST' }));
    expect(store.cleared).toBe(true);
  });

  it('flags responses that need a passkey-verified session', async () => {
    const fetcher = vi.fn().mockResolvedValue(json({ error: { code: 'mfa_required', message: 'verify with a passkey to continue' } }, 403));
    const api = new ApiClient('/api', fetcher, false);
    api.setToken('t');
    await expect(api.adminUsers()).rejects.toMatchObject({ status: 403, code: 'mfa_required' });
  });
});
