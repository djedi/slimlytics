import { describe, expect, it, vi } from 'vitest';
import { ApiClient, type SessionStore } from '../src/lib/api';

function json(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
}

/** A JWT-shaped token for `sub`; only the payload matters to the client. */
function jwt(sub: string, nonce = Math.random().toString(36).slice(2)) {
  const part = (value: object) => btoa(JSON.stringify(value)).replace(/=+$/, '').replace(/\+/g, '-').replace(/\//g, '_');
  return `${part({ alg: 'HS256' })}.${part({ sub, sid: nonce, exp: 1 })}.sig`;
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
    const stale = jwt('user-a');
    const fromOtherTab = jwt('user-a');
    const store = memoryStore(stale, 'slrt_first');
    const fetcher = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url.endsWith('/auth/refresh')) throw new Error('should not refresh');
      const auth = (init?.headers as Record<string, string>).authorization;
      if (auth === `Bearer ${stale}`) {
        store.state.token = fromOtherTab;
        store.state.refreshToken = 'slrt_other';
        return json({}, 401);
      }
      return json([]);
    });
    const api = new ApiClient('/api', fetcher, false);
    api.useSession(store);
    await expect(api.sites()).resolves.toEqual([]);
    expect(api.accessToken).toBe(fromOtherTab);
  });

  it('never retries a request as a different account another tab signed in to', async () => {
    const accountA = jwt('user-a');
    const accountB = jwt('user-b');
    const store = memoryStore(accountA, 'slrt_a');
    const fetcher = vi.fn(async (input: RequestInfo | URL) => {
      if (String(input).endsWith('/auth/refresh')) throw new Error('must not spend account B\'s refresh token');
      // Meanwhile another tab signed in as account B.
      store.state.token = accountB;
      store.state.refreshToken = 'slrt_b';
      return json({}, 401);
    });
    const api = new ApiClient('/api', fetcher, false);
    api.useSession(store);
    const unauthorized = vi.fn();
    api.onUnauthorized = unauthorized;
    await expect(api.createSite({ name: 'A site', domain: 'a.example.com' })).rejects.toMatchObject({ status: 401 });
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(unauthorized).toHaveBeenCalledOnce();
    expect(store.state.token).toBe(accountB);
  });

  it('waits for an in-flight refresh before signing out so the new session is revoked', async () => {
    const store = memoryStore(jwt('user-a'), 'slrt_old');
    let finishRefresh: (response: Response) => void = () => {};
    const fetcher = vi.fn((input: RequestInfo | URL, _init?: RequestInit) => {
      const url = String(input);
      if (url.endsWith('/auth/refresh')) return new Promise<Response>((resolve) => (finishRefresh = resolve));
      if (url.endsWith('/auth/logout')) return Promise.resolve(new Response(null, { status: 204 }));
      return Promise.resolve(json({}, 401));
    });
    const api = new ApiClient('/api', fetcher, false);
    api.useSession(store);
    const pending = api.sites().catch(() => undefined);
    await vi.waitFor(() => expect(fetcher).toHaveBeenCalledWith('/api/auth/refresh', expect.anything()));
    const signingOut = api.logout();
    finishRefresh(json({ token: jwt('user-a'), refreshToken: 'slrt_new', expiresIn: 3600 }));
    await signingOut;
    await pending;
    const logout = fetcher.mock.calls.find(([url]) => String(url).endsWith('/auth/logout'));
    expect(logout?.[1]?.body).toBe(JSON.stringify({ refreshToken: 'slrt_new' }));
    expect(store.state).toEqual({ token: '', refreshToken: '' });
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
    // The refresh token identifies the session even if the access token has expired.
    expect(fetcher).toHaveBeenLastCalledWith('/api/auth/logout', expect.objectContaining({
      method: 'POST',
      body: JSON.stringify({ refreshToken: 'slrt_a' })
    }));
    expect(store.cleared).toBe(true);
  });

  it('flags responses that need a passkey-verified session', async () => {
    const fetcher = vi.fn().mockResolvedValue(json({ error: { code: 'mfa_required', message: 'verify with a passkey to continue' } }, 403));
    const api = new ApiClient('/api', fetcher, false);
    api.setToken('t');
    await expect(api.adminUsers()).rejects.toMatchObject({ status: 403, code: 'mfa_required' });
  });

  it('signing out a stale tab leaves another account\'s session alone', async () => {
    const accountA = jwt('user-a');
    const accountB = jwt('user-b');
    const store = memoryStore(accountA, 'slrt_a');
    const fetcher = vi.fn().mockResolvedValue(new Response(null, { status: 204 }));
    const api = new ApiClient('/api', fetcher, false);
    api.useSession(store);
    Object.assign(store.state, { token: accountB, refreshToken: 'slrt_b' });
    await api.logout();
    expect(fetcher).toHaveBeenCalledWith('/api/auth/logout', expect.objectContaining({
      body: '{}',
      headers: expect.objectContaining({ authorization: `Bearer ${accountA}` })
    }));
    expect(store.state).toEqual({ token: accountB, refreshToken: 'slrt_b' });
  });

  it('forgetting a rejected session keeps a newer account in shared storage', () => {
    const store = memoryStore(jwt('user-a'), 'slrt_a');
    const api = new ApiClient('/api', vi.fn(), false);
    api.useSession(store);
    const accountB = jwt('user-b');
    Object.assign(store.state, { token: accountB, refreshToken: 'slrt_b' });
    api.forgetSession();
    expect(store.state.token).toBe(accountB);
    expect(store.cleared).toBe(false);
    api.useSession(store);
    api.forgetSession();
    expect(store.cleared).toBe(true);
  });

  it('discards and revokes a sign-in that finishes after signing out', async () => {
    let finishLogin: (response: Response) => void = () => {};
    const fetcher = vi.fn((input: RequestInfo | URL, _init?: RequestInit) =>
      String(input).endsWith('/auth/login')
        ? new Promise<Response>((resolve) => (finishLogin = resolve))
        : Promise.resolve(new Response(null, { status: 204 }))
    );
    const store = memoryStore('', '');
    const api = new ApiClient('/api', fetcher, false);
    api.useSession(store);
    const signingIn = api.login('a@example.com', 'correct horse battery staple');
    await vi.waitFor(() => expect(fetcher).toHaveBeenCalledOnce());
    await api.logout();
    finishLogin(json({ token: jwt('user-a'), refreshToken: 'slrt_late', expiresIn: 3600 }));
    await expect(signingIn).rejects.toMatchObject({ status: 401 });
    expect(store.state).toEqual({ token: '', refreshToken: '' });
    expect(fetcher).toHaveBeenLastCalledWith('/api/auth/logout', expect.objectContaining({ body: JSON.stringify({ refreshToken: 'slrt_late' }) }));
  });

  it('never lets a refresh response overwrite a session that changed meanwhile', async () => {
    const accountA = jwt('user-a');
    const store = memoryStore(accountA, 'slrt_a');
    const fetcher = vi.fn(async (input: RequestInfo | URL, _init?: RequestInit) => {
      const url = String(input);
      if (url.endsWith('/auth/refresh')) {
        Object.assign(store.state, { token: jwt('user-b'), refreshToken: 'slrt_b' });
        return json({ token: jwt('user-a'), refreshToken: 'slrt_a2', expiresIn: 3600 });
      }
      if (url.endsWith('/auth/logout')) return new Response(null, { status: 204 });
      return json({}, 401);
    });
    const api = new ApiClient('/api', fetcher, false);
    api.useSession(store);
    await expect(api.sites()).rejects.toMatchObject({ status: 401 });
    expect(store.state.refreshToken).toBe('slrt_b');
    expect(fetcher).toHaveBeenCalledWith('/api/auth/logout', expect.objectContaining({ body: JSON.stringify({ refreshToken: 'slrt_a2' }) }));
  });
});
