export interface TrendPoint { date: string; visitors: number; pageViews: number }
export interface Overview { visitors: number; previousVisitors?: number; sessions: number; pageViews: number; bounceRate: number; avgDuration: number; change: number; currentOnline: number; trend: TrendPoint[] }
export type AntiAdblockServer = 'caddy' | 'nginx' | 'apache';
export interface AntiAdblockSettings { serverType: AntiAdblockServer; jsPath: string; beaconPath: string }
export interface BillingPlan { id: string; name: string; sites: number | null; dailyPageViews: number | null; monthlyPriceCents: number; annualPriceCents: number; currency: string; intervals?: ('month' | 'year')[] }
export interface BillingStatus {
  enabled: boolean;
  plan?: BillingPlan;
  planSource?: 'default' | 'stripe' | 'admin';
  subscriptionStatus?: string | null;
  interval?: 'month' | 'year' | null;
  currentPeriodEnd?: string | null;
  hasBillingAccount?: boolean;
  usage?: { sites: number; pageViewsToday: number };
  plans?: BillingPlan[];
  checkoutAvailable?: boolean;
}
export interface Site { id: string; name: string; domain: string; writeKey: string; serverWriteKey: string; proxyKey?: string; timezone?: string; allowedOrigins?: string[]; retentionDays?: number; antiAdblockServer: AntiAdblockServer; antiAdblockJsPath: string; antiAdblockBeaconPath: string; overview?: Overview }
export interface User { id: string; email: string; name?: string; createdAt?: string; isAdmin?: boolean; mfaVerified?: boolean; passkeyCount?: number }
export interface AuthResponse { token?: string; accessToken?: string; refreshToken?: string; expiresIn?: number; user?: User }
export interface SessionTokens { token: string; refreshToken: string }
/** Where the access and refresh tokens live between page loads. */
export interface SessionStore { load(): SessionTokens; save(tokens: SessionTokens): void; clear(): void }
export interface AccountSession { id: string; authMethod: 'password' | 'passkey'; userAgent?: string | null; createdAt: string; lastUsedAt: string; expiresAt: string; mfaVerified: boolean; current: boolean }
export interface PasskeySummary { id: string; name: string; createdAt: string; lastUsedAt?: string | null }
export interface WebAuthnChallenge { challengeId: string; options: Record<string, unknown> }
export interface AdminOverview { users: number; admins: number; disabledUsers: number; signupsLast7Days: number; sites: number; activeUsersLast7Days: number }
export interface AdminUser { id: string; email: string; createdAt: string; lastLoginAt?: string | null; disabledAt?: string | null; isAdmin: boolean; siteCount: number; passkeyCount: number; activeSessions: number; plan?: string | null }
export interface AdminUserDetail {
  user: { id: string; email: string; createdAt: string; lastLoginAt?: string | null; disabledAt?: string | null; isAdmin: boolean; plan?: string | null; subscriptionStatus?: string | null };
  sites: { id: string; name: string; domain: string; role: string; createdAt: string }[];
  sessions: { id: string; authMethod: string; userAgent?: string | null; createdAt: string; lastUsedAt: string; expiresAt: string }[];
  passkeys: PasskeySummary[];
  activeApiTokens: number;
}
export interface AdminAuditEntry { id: number; actorEmail: string; action: string; targetUserId?: string | null; targetEmail?: string | null; metadata: Record<string, unknown>; createdAt: string }

const TOKEN_KEY = 'slimlytics_token';
const REFRESH_KEY = 'slimlytics_refresh_token';
/** The browser's session: shared by every tab through localStorage. */
export const browserSession: SessionStore = {
  load: () => ({ token: localStorage.getItem(TOKEN_KEY) ?? '', refreshToken: localStorage.getItem(REFRESH_KEY) ?? '' }),
  save: ({ token, refreshToken }) => {
    localStorage.setItem(TOKEN_KEY, token);
    localStorage.setItem(REFRESH_KEY, refreshToken);
  },
  clear: () => {
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(REFRESH_KEY);
  }
};

const SESSION_LOCK = 'slimlytics-session';
let localQueue: Promise<unknown> = Promise.resolve();
/**
 * Serializes refresh and sign-out across tabs (Web Locks) or, without that API, within this
 * page, so two refreshes never spend the same refresh token and a refresh that is still in
 * flight can never restore a session after sign-out.
 */
async function withSessionLock<T>(task: () => Promise<T>): Promise<T> {
  const locks = typeof navigator === 'undefined' ? undefined : (navigator as Navigator & { locks?: LockManager }).locks;
  if (locks) return locks.request(SESSION_LOCK, task);
  const run = localQueue.then(task, task);
  localQueue = run.catch(() => undefined);
  return run;
}

/** The account a session JWT belongs to (its `sub` claim), or undefined if unreadable. */
export function tokenSubject(token: string): string | undefined {
  try {
    const payload = token.split('.')[1];
    if (!payload) return undefined;
    const json = atob(payload.replace(/-/g, '+').replace(/_/g, '/'));
    const sub = (JSON.parse(json) as { sub?: unknown }).sub;
    return typeof sub === 'string' ? sub : undefined;
  } catch {
    return undefined;
  }
}

// Requests that must never trigger a refresh-and-retry.
const sessionlessPaths = ['/auth/login', '/auth/register', '/auth/refresh', '/auth/logout', '/auth/passkey/'];
export interface ReportRow { label: string; value: number; visitors?: number; secondary?: string; change?: number }
export interface Visitor { id: string; country: string; region?: string; city?: string; device?: string; browser?: string; page?: string; lastSeen?: string; sessions?: number }
export interface LiveEvent { id: string; type: string; page: string; visitorId?: string; country?: string; city?: string; timestamp: string; referrer?: string }
export interface Goal { id: string; name: string; type: string; target: string; conversions?: number; conversionRate?: number }
export interface CollectionHealth {
  acceptedTotal: number;
  rejectedTotal: number;
  lastAcceptedAt?: string | null;
  lastRejectedAt?: string | null;
  lastRejectionCode?: string | null;
  lastTrackerVersion?: string | null;
}
export interface Journey { steps: string[]; sessions: number; visitors: number }
export interface Attribution {
  source: string;
  medium: string;
  campaign: string;
  visitors: number;
  conversions: number;
  revenue: number;
}
export interface Anomaly {
  date: string;
  metric: 'pageViews';
  value: number;
  baseline: number;
  deviationPercent: number;
  direction: 'up' | 'down';
}
export interface Annotation { id: string; siteId: string; occurredOn: string; label: string; createdAt: string }
export interface FunnelStep { label: string; eventName?: string; path?: string }
export interface Funnel { id: string; name: string; steps: FunnelStep[]; createdAt: string }
export interface FunnelReportStep { index: number; label: string; visitors: number; conversionRate: number }
export interface FunnelReport { id: string; name: string; from: string; to: string; steps: FunnelReportStep[] }
export interface SearchConsoleStatus {
  configured: boolean;
  connected: boolean;
  propertyUrl?: string | null;
  lastSyncedAt?: string | null;
  lastError?: string | null;
}
export interface SearchConsoleRow {
  value: string;
  clicks: number;
  impressions: number;
  ctr: number;
  position: number;
}
export interface ReportSubscription {
  id: string; siteId: string; name: string; webhookUrl: string; frequency: 'daily' | 'weekly';
  anomalyOnly: boolean; enabled: boolean; nextRunAt: string; lastSentAt?: string | null;
  lastStatus?: 'success' | 'error' | 'skipped' | null; lastError?: string | null; createdAt: string;
  signingSecret?: string;
}

export class ApiError extends Error {
  constructor(public status: number, message: string, public code?: string) { super(message); this.name = 'ApiError'; }
}

type Fetcher = typeof fetch;
interface WireSite extends Omit<Site, 'writeKey' | 'serverWriteKey' | 'antiAdblockServer' | 'antiAdblockJsPath' | 'antiAdblockBeaconPath'> { write_key?: string; writeKey?: string; server_write_key?: string; serverWriteKey?: string; antiAdblockServer?: AntiAdblockServer; anti_adblock_server?: AntiAdblockServer; antiAdblockJsPath?: string; anti_adblock_js_path?: string; antiAdblockBeaconPath?: string; anti_adblock_beacon_path?: string }
interface WireMetric { current: number; previous: number; change_percent?: number | null }
interface WireOverview {
  views: WireMetric;
  visitors: WireMetric;
  sessions: WireMetric;
  events: WireMetric;
  currentOnline?: number;
  current_online?: number;
  bounceRate?: number;
  bounce_rate?: number;
  avgDurationSeconds?: number;
  avg_duration_seconds?: number;
  trend?: TrendPoint[];
}
interface WireReportRow { value: string; views: number; visitors: number }
interface WireGoal { id: string; name: string; eventName?: string; event_name?: string; pathPattern?: string; path_pattern?: string; conversions?: number; conversionRate?: number; conversion_rate?: number }
function dates(days: number) {
  const to = new Date();
  const from = new Date(to.getTime() - Math.max(0, days - 1) * 864e5);
  return { from: from.toISOString().slice(0, 10), to: to.toISOString().slice(0, 10) };
}
function dateQuery(days: number) { const range = dates(days); return `from=${range.from}&to=${range.to}`; }
function normalizeSite(site: WireSite): Site { return { ...site, writeKey: site.writeKey ?? site.write_key ?? '', serverWriteKey: site.serverWriteKey ?? site.server_write_key ?? '', antiAdblockServer: site.antiAdblockServer ?? site.anti_adblock_server ?? 'caddy', antiAdblockJsPath: site.antiAdblockJsPath ?? site.anti_adblock_js_path ?? '/slimlytics.js', antiAdblockBeaconPath: site.antiAdblockBeaconPath ?? site.anti_adblock_beacon_path ?? '/slimlytics-event' }; }
const trend = Array.from({ length: 28 }, (_, index) => ({ date: new Date(Date.now() - (27 - index) * 864e5).toISOString().slice(0, 10), visitors: 84 + ((index * 17) % 71), pageViews: 151 + ((index * 29) % 129) }));
const baseOverview: Overview = { visitors: 3421, sessions: 3892, pageViews: 8754, bounceRate: 38.4, avgDuration: 164, change: 12.8, currentOnline: 14, trend };
export const demoSites: Site[] = [
  { id: 'northstar', name: 'Northstar Docs', domain: 'docs.northstar.dev', writeKey: 'wk_demo_docs', serverWriteKey: 'swk_demo_docs', antiAdblockServer: 'caddy', antiAdblockJsPath: '/a4f20197c631.js', antiAdblockBeaconPath: '/39dab7e081b2', overview: baseOverview },
  { id: 'journal', name: 'Field Journal', domain: 'journal.example.com', writeKey: 'wk_demo_journal', serverWriteKey: 'swk_demo_journal', antiAdblockServer: 'nginx', antiAdblockJsPath: '/b72d91a0f442.js', antiAdblockBeaconPath: '/41f8c02be992', overview: { ...baseOverview, visitors: 1886, pageViews: 5102, currentOnline: 6, change: 4.2 } },
  { id: 'store', name: 'Little Supply Co.', domain: 'shop.example.com', writeKey: 'wk_demo_shop', serverWriteKey: 'swk_demo_shop', antiAdblockServer: 'apache', antiAdblockJsPath: '/c03385e781a9.js', antiAdblockBeaconPath: '/62dea9430bc1', overview: { ...baseOverview, visitors: 956, pageViews: 2901, currentOnline: 2, change: -3.7 } }
];

const reportLabels: Record<string, string[]> = {
  pages: ['/docs/getting-started', '/', '/pricing', '/blog/privacy-first-analytics', '/integrations'],
  referrers: ['Google', 'Direct / none', 'github.com', 'Bing', 'newsletter'],
  countries: ['United States', 'Germany', 'United Kingdom', 'Canada', 'Japan'],
  regions: ['Colorado', 'California', 'New York', 'Ontario', 'Bavaria'],
  cities: ['Denver', 'San Francisco', 'New York', 'Toronto', 'Munich'],
  devices: ['Desktop · Chrome', 'Mobile · Safari', 'Desktop · Firefox', 'Mobile · Chrome', 'Tablet · Safari'],
  browsers: ['Chrome', 'Safari', 'Firefox', 'Edge', 'Brave'],
  'operating-systems': ['Windows', 'Mac OSX', 'iOS', 'Android', 'Linux'],
  campaigns: ['spring_launch / email', 'docs_sidebar / referral', 'brand / search', 'july_digest / email', '(not set)']
};
export const demoReport = (type: string): ReportRow[] => (reportLabels[type] ?? []).map((label, i) => ({ label, value: 1420 - i * 219, secondary: `${Math.max(5, 38 - i * 6)}%`, change: i % 2 ? -2.1 : 8.4 }));

export class ApiClient {
  private token = '';
  private base: string;
  private store?: SessionStore;
  private refreshing?: Promise<boolean>;
  // Bumped on sign-out so a sign-in response that arrives afterwards is discarded.
  private generation = 0;
  constructor(base = '/api', private fetcher: Fetcher = fetch, private demo = false) { this.base = base.replace(/\/$/, ''); }
  onUnauthorized?: () => void;
  setToken(token: string) { this.token = token; }
  /** Keeps the session in `store` and refreshes expired access tokens transparently. */
  useSession(store: SessionStore) {
    this.store = store;
    this.token = store.load().token;
  }
  get accessToken() { return this.token; }
  /** Stores a new sign-in, serialized with refresh and sign-out. */
  private async adopt(response: AuthResponse, generation: number) {
    const token = response.accessToken ?? response.token ?? '';
    const refreshToken = response.refreshToken;
    if (!token || !refreshToken || !this.store) {
      if (token) this.token = token;
      return response;
    }
    await withSessionLock(async () => {
      if (generation !== this.generation) {
        // Signed out while this sign-in was in flight: end the session it just created.
        await this.revokeByRefreshToken(refreshToken);
        throw new ApiError(401, 'You signed out before sign-in finished.');
      }
      this.store!.save({ token, refreshToken });
      this.token = token;
    });
    return response;
  }
  /** Best-effort server sign-out of a session nobody will use. */
  private async revokeByRefreshToken(refreshToken: string) {
    try {
      await this.fetcher(`${this.base}/auth/logout`, {
        method: 'POST',
        headers: { accept: 'application/json', 'content-type': 'application/json' },
        body: JSON.stringify({ refreshToken })
      });
    } catch { /* it expires on its own */ }
  }
  /**
   * Forgets this client's rejected session. Shared storage is cleared only if it still holds
   * that session, so a stale tab never signs out an account another tab signed in to.
   */
  forgetSession() {
    const stored = this.store?.load();
    if (stored && (!stored.token || stored.token === this.token)) this.store!.clear();
    this.token = '';
  }
  /**
   * Gets a new access token after `failedToken` was rejected. Resolves false when the
   * session is over (signed out, revoked, or expired); throws on network or server errors
   * so a flaky connection never signs anyone out.
   */
  refreshSession(failedToken = this.token): Promise<boolean> {
    if (!this.store || this.demo) return Promise.resolve(false);
    this.refreshing ??= withSessionLock(() => this.exchangeRefreshToken(failedToken)).finally(() => {
      this.refreshing = undefined;
    });
    return this.refreshing;
  }
  private async exchangeRefreshToken(failedToken: string) {
    const store = this.store!;
    const current = store.load();
    if (current.token && current.token !== failedToken) {
      // Another tab changed the session while this one waited. Adopt it only if it is the
      // same account; otherwise this tab is stale and must not act as someone else.
      const subject = tokenSubject(failedToken);
      if (!subject || subject !== tokenSubject(current.token)) return false;
      this.token = current.token;
      return true;
    }
    if (!current.refreshToken) return false;
    const response = await this.fetcher(`${this.base}/auth/refresh`, {
      method: 'POST',
      headers: { accept: 'application/json', 'content-type': 'application/json' },
      body: JSON.stringify({ refreshToken: current.refreshToken })
    });
    if (response.status === 400 || response.status === 401) {
      if (store.load().refreshToken === current.refreshToken) store.clear();
      return false;
    }
    if (!response.ok) throw new ApiError(response.status, `Could not refresh your session (${response.status})`);
    const tokens = await response.json() as SessionTokens;
    // Storage changed underneath (e.g. a tab without the lock): never overwrite it.
    if (store.load().refreshToken !== current.refreshToken) {
      await this.revokeByRefreshToken(tokens.refreshToken);
      return false;
    }
    store.save({ token: tokens.token, refreshToken: tokens.refreshToken });
    this.token = tokens.token;
    return true;
  }
  // `attempts` counts refreshes for this request: a token adopted from another tab may itself
  // have expired, so one more refresh is allowed before the session counts as over.
  private async request<T>(path: string, init: RequestInit = {}, fallback?: () => T, attempts = 0): Promise<T> {
    const sent = this.token;
    const headers: Record<string, string> = { accept: 'application/json', ...(init.body ? { 'content-type': 'application/json' } : {}), ...(sent ? { authorization: `Bearer ${sent}` } : {}), ...(init.headers as Record<string, string> ?? {}) };
    try {
      const response = await this.fetcher(`${this.base}${path}`, { ...init, headers });
      if (!response.ok) {
        if (response.status === 401 && sent && !this.demo && !sessionlessPaths.some((prefix) => path.startsWith(prefix))) {
          if (attempts < 2 && await this.refreshSession(sent)) return this.request(path, init, fallback, attempts + 1);
          this.onUnauthorized?.();
        }
        let message = `Request failed (${response.status})`;
        let code: string | undefined;
        try {
          const body = await response.json() as { message?: string; error?: string | { message?: string; code?: string } };
          message = body.message ?? (typeof body.error === 'string' ? body.error : body.error?.message) ?? message;
          code = typeof body.error === 'object' ? body.error?.code : undefined;
        } catch { /* non-JSON */ }
        throw new ApiError(response.status, message, code);
      }
      if (response.status === 204) return undefined as T;
      return await response.json() as T;
    } catch (error) {
      if (error instanceof ApiError || !this.demo || !fallback) throw error;
      return structuredClone(fallback());
    }
  }
  billing() { return this.request<BillingStatus>('/billing', {}, () => ({ enabled: false })); }
  billingCheckout(plan: string, interval: 'month' | 'year' = 'month') { return this.request<{ url: string; portal: boolean }>('/billing/checkout', { method: 'POST', body: JSON.stringify({ plan, interval }) }); }
  billingPortal() { return this.request<{ url: string }>('/billing/portal', { method: 'POST' }); }
  async register(email: string, password: string, name = '') {
    const generation = this.generation;
    return this.adopt(await this.request<AuthResponse>('/auth/register', { method: 'POST', body: JSON.stringify({ email, password, name }) }), generation);
  }
  async login(email: string, password: string) {
    const generation = this.generation;
    return this.adopt(await this.request<AuthResponse>('/auth/login', { method: 'POST', body: JSON.stringify({ email, password }) }, () => ({ accessToken: 'demo', user: { id: 'demo', email } })), generation);
  }
  /** Ends this session on the server, then forgets it locally even if the server is unreachable. */
  async logout() {
    this.generation += 1;
    // Under the session lock, so any refresh in flight (here or in another tab) finishes
    // first and the session it renewed is the one revoked.
    await withSessionLock(async () => {
      const stored = this.store?.load();
      // Storage holding a different account belongs to another tab: leave it alone and
      // revoke only this client's own session.
      const subject = tokenSubject(this.token);
      const mine = !!stored && (!this.token || stored.token === this.token || (!!subject && tokenSubject(stored.token) === subject));
      const token = mine ? stored!.token || this.token : this.token;
      const refreshToken = mine ? stored!.refreshToken : '';
      try {
        if ((token || refreshToken) && !this.demo)
          await this.fetcher(`${this.base}/auth/logout`, {
            method: 'POST',
            headers: {
              accept: 'application/json',
              'content-type': 'application/json',
              ...(token ? { authorization: `Bearer ${token}` } : {})
            },
            body: JSON.stringify(refreshToken ? { refreshToken } : {})
          });
      } catch { /* already signed out */ } finally {
        if (mine) this.store?.clear();
        this.token = '';
      }
    });
  }
  me() { return this.request<User>('/auth/me'); }
  startPasskeySignIn() { return this.request<WebAuthnChallenge>('/auth/passkey/start', { method: 'POST' }); }
  async finishPasskeySignIn(challengeId: string, credential: unknown) {
    const generation = this.generation;
    return this.adopt(await this.request<AuthResponse>('/auth/passkey/finish', { method: 'POST', body: JSON.stringify({ challengeId, credential }) }), generation);
  }
  startStepUp() { return this.request<WebAuthnChallenge>('/auth/mfa/start', { method: 'POST' }); }
  finishStepUp(challengeId: string, credential: unknown) { return this.request<void>('/auth/mfa/finish', { method: 'POST', body: JSON.stringify({ challengeId, credential }) }); }
  accountSessions() { return this.request<AccountSession[]>('/account/sessions'); }
  revokeAccountSession(id: string) { return this.request<void>(`/account/sessions/${id}`, { method: 'DELETE' }); }
  passkeys() { return this.request<PasskeySummary[]>('/account/passkeys'); }
  startPasskeyRegistration(currentPassword?: string) { return this.request<WebAuthnChallenge>('/account/passkeys/register/start', { method: 'POST', body: JSON.stringify(currentPassword ? { currentPassword } : {}) }); }
  finishPasskeyRegistration(challengeId: string, name: string, credential: unknown) { return this.request<PasskeySummary>('/account/passkeys/register/finish', { method: 'POST', body: JSON.stringify({ challengeId, name, credential }) }); }
  deletePasskey(id: string) { return this.request<void>(`/account/passkeys/${id}`, { method: 'DELETE' }); }
  adminOverview() { return this.request<AdminOverview>('/admin/overview'); }
  adminUsers(query = '', offset = 0, limit = 50) { return this.request<{ users: AdminUser[]; total: number }>(`/admin/users?${new URLSearchParams({ q: query, offset: String(offset), limit: String(limit) })}`); }
  adminUser(id: string) { return this.request<AdminUserDetail>(`/admin/users/${id}`); }
  adminUserAction(id: string, action: 'disable' | 'enable' | 'revoke-sessions') { return this.request<void>(`/admin/users/${id}/${action}`, { method: 'POST' }); }
  adminDeleteUser(id: string, confirmEmail: string) { return this.request<void>(`/admin/users/${id}`, { method: 'DELETE', body: JSON.stringify({ confirmEmail }) }); }
  adminAudit(limit = 100) { return this.request<AdminAuditEntry[]>(`/admin/audit?limit=${limit}`); }
  async sites() { return (await this.request<WireSite[]>('/sites', {}, () => demoSites)).map(normalizeSite); }
  async site(id: string) { return normalizeSite(await this.request<WireSite>(`/sites/${id}`, {}, () => demoSites.find((site) => site.id === id) ?? demoSites[0])); }
  async createSite(site: Pick<Site, 'name' | 'domain'> & { allowedOrigins?: string[] }) {
    return normalizeSite(await this.request<WireSite>('/sites', { method: 'POST', body: JSON.stringify({ ...site, timezone: 'UTC', retentionDays: 365 }) }, () => ({ ...site, id: crypto.randomUUID(), writeKey: `wk_demo_${Date.now()}` })));
  }
  async updateSite(id: string, site: Partial<Site>) { return normalizeSite(await this.request<WireSite>(`/sites/${id}`, { method: 'PUT', body: JSON.stringify(site) }, () => ({ ...demoSites[0], ...site, id }))); }
  async updateAntiAdblock(id: string, settings: AntiAdblockSettings) { return normalizeSite(await this.request<WireSite>(`/sites/${id}/anti-adblock`, { method: 'PUT', body: JSON.stringify(settings) }, () => ({ ...demoSites[0], id, antiAdblockServer: settings.serverType, antiAdblockJsPath: settings.jsPath, antiAdblockBeaconPath: settings.beaconPath }))); }
  rotateProxyKey(id: string) { return this.request<{ proxyKey: string }>(`/sites/${id}/rotate-proxy-key`, { method: 'POST' }); }
  rotateServerKey(id: string) { return this.request<{ serverWriteKey: string }>(`/sites/${id}/rotate-server-key`, { method: 'POST' }); }
  collectionHealth(id: string) { return this.request<CollectionHealth>(`/sites/${id}/collection-health`, {}, () => ({ acceptedTotal: 0, rejectedTotal: 0 })); }
  deleteSite(id: string) { return this.request<void>(`/sites/${id}`, { method: 'DELETE' }, () => undefined); }
  async overview(id: string, days = 28): Promise<Overview> {
    const wire = await this.request<WireOverview>(`/sites/${id}/overview?${dateQuery(days)}`, {}, () => ({
      views: { current: baseOverview.pageViews, previous: 0 },
      visitors: { current: baseOverview.visitors, previous: 0 },
      sessions: { current: baseOverview.sessions, previous: 0 },
      events: { current: 0, previous: 0 },
      currentOnline: baseOverview.currentOnline
    }));
    const change = wire.visitors.change_percent ?? wire.views.change_percent ?? 0;
    return {
      visitors: wire.visitors.current,
      previousVisitors: wire.visitors.previous,
      sessions: wire.sessions.current,
      pageViews: wire.views.current,
      bounceRate: wire.bounceRate ?? wire.bounce_rate ?? 0,
      avgDuration: wire.avgDurationSeconds ?? wire.avg_duration_seconds ?? 0,
      change,
      currentOnline: wire.currentOnline ?? wire.current_online ?? 0,
      trend: wire.trend ?? []
    };
  }
  async report(id: string, type: string, days = 28) { const rows = await this.request<WireReportRow[]>(`/sites/${id}/reports/${type}?${dateQuery(days)}`, {}, () => []); return rows.map((row) => ({ label: row.value, value: row.views, visitors: row.visitors })); }
  journeys(id: string, days = 28) { return this.request<Journey[]>(`/sites/${id}/insights/journeys?${dateQuery(days)}`, {}, () => []); }
  attribution(id: string, days = 28) { return this.request<Attribution[]>(`/sites/${id}/insights/attribution?${dateQuery(days)}`, {}, () => []); }
  anomalies(id: string, days = 28) { return this.request<Anomaly[]>(`/sites/${id}/insights/anomalies?${dateQuery(days)}`, {}, () => []); }
  annotations(id: string, days = 28) { return this.request<Annotation[]>(`/sites/${id}/annotations?${dateQuery(days)}`, {}, () => []); }
  createAnnotation(id: string, annotation: Pick<Annotation, 'occurredOn' | 'label'>) { return this.request<Annotation>(`/sites/${id}/annotations`, { method: 'POST', body: JSON.stringify(annotation) }); }
  deleteAnnotation(id: string, annotationId: string) { return this.request<void>(`/sites/${id}/annotations/${annotationId}`, { method: 'DELETE' }); }
  funnels(id: string) { return this.request<Funnel[]>(`/sites/${id}/funnels`, {}, () => []); }
  createFunnel(id: string, funnel: Pick<Funnel, 'name' | 'steps'>) { return this.request<Funnel>(`/sites/${id}/funnels`, { method: 'POST', body: JSON.stringify(funnel) }); }
  deleteFunnel(id: string, funnelId: string) { return this.request<void>(`/sites/${id}/funnels/${funnelId}`, { method: 'DELETE' }); }
  funnelReport(id: string, funnelId: string, days = 28) { return this.request<FunnelReport>(`/sites/${id}/funnels/${funnelId}/report?${dateQuery(days)}`, {}, () => ({ id: funnelId, name: 'Funnel', from: dates(days).from, to: dates(days).to, steps: [] })); }
  searchConsoleStatus(id: string) { return this.request<SearchConsoleStatus>(`/sites/${id}/integrations/search-console`, {}, () => ({ configured: false, connected: false })); }
  connectSearchConsole(id: string) { return this.request<{ authorizationUrl: string }>(`/sites/${id}/integrations/search-console/connect`, { method: 'POST' }); }
  disconnectSearchConsole(id: string) { return this.request<void>(`/sites/${id}/integrations/search-console`, { method: 'DELETE' }); }
  syncSearchConsole(id: string, days = 28) { return this.request<{ status: string; rows: number }>(`/sites/${id}/integrations/search-console/sync?${dateQuery(days)}`, { method: 'POST' }); }
  searchConsoleReport(id: string, dimension: 'query' | 'page' | 'country' | 'device' | 'date' = 'query', days = 28) { return this.request<SearchConsoleRow[]>(`/sites/${id}/reports/search-console?${dateQuery(days)}&dimension=${dimension}`, {}, () => []); }
  reportSubscriptions(id: string) { return this.request<ReportSubscription[]>(`/sites/${id}/report-subscriptions`, {}, () => []); }
  createReportSubscription(id: string, subscription: Omit<ReportSubscription, 'id' | 'siteId' | 'nextRunAt' | 'createdAt' | 'lastSentAt' | 'lastStatus' | 'lastError' | 'signingSecret'>) { return this.request<ReportSubscription>(`/sites/${id}/report-subscriptions`, { method: 'POST', body: JSON.stringify(subscription) }); }
  updateReportSubscription(id: string, subscription: ReportSubscription) { return this.request<ReportSubscription>(`/sites/${id}/report-subscriptions/${subscription.id}`, { method: 'PUT', body: JSON.stringify(subscription) }); }
  deleteReportSubscription(id: string, subscriptionId: string) { return this.request<void>(`/sites/${id}/report-subscriptions/${subscriptionId}`, { method: 'DELETE' }); }
  deliverReportSubscription(id: string, subscriptionId: string) { return this.request<{ status: string }>(`/sites/${id}/report-subscriptions/${subscriptionId}/deliver`, { method: 'POST' }); }
  visitors(id: string, days = 28) { return this.request<Visitor[]>(`/sites/${id}/visitors?${dateQuery(days)}`, {}, () => [{ id: 'v1', country: 'United States', city: 'Portland', device: 'Desktop', browser: 'Firefox', page: '/docs', sessions: 4, lastSeen: new Date().toISOString() }]); }
  events(id: string, days = 28) { return this.request<LiveEvent[]>(`/sites/${id}/events?${dateQuery(days)}`, {}, () => [{ id: 'e1', type: 'pageview', page: '/docs/getting-started', visitorId: 'v1', country: 'US', city: 'Portland', timestamp: new Date().toISOString(), referrer: 'Google' }]); }
  async goals(id: string) { const rows = await this.request<WireGoal[]>(`/sites/${id}/goals`, {}, () => []); return rows.map((goal) => ({ id: goal.id, name: goal.name, type: 'event', target: goal.eventName ?? goal.event_name ?? '', conversions: goal.conversions ?? 0, conversionRate: goal.conversionRate ?? goal.conversion_rate ?? 0 })); }
  async createGoal(id: string, goal: Omit<Goal, 'id'>) { const row = await this.request<WireGoal>(`/sites/${id}/goals`, { method: 'POST', body: JSON.stringify({ name: goal.name, eventName: goal.target }) }, () => ({ ...goal, id: crypto.randomUUID(), eventName: goal.target })); return { id: row.id, name: row.name, type: 'event', target: row.eventName ?? row.event_name ?? goal.target, conversions: 0, conversionRate: 0 }; }
  exportUrl(id: string, days: number) { return `${this.base}/sites/${id}/export.csv?${dateQuery(days)}`; }
  streamUrl(id: string, token?: string) { const query = token ? `?token=${encodeURIComponent(token)}` : ''; return `${this.base}/sites/${id}/stream${query}`; }
  async downloadExport(id: string, days: number) {
    const response = await this.fetcher(`${this.base}/sites/${id}/export.csv?${dateQuery(days)}`, { headers: this.token ? { authorization: `Bearer ${this.token}` } : {} });
    if (response.status === 401 && this.token && !this.demo) this.onUnauthorized?.();
    if (!response.ok) throw new ApiError(response.status, `Export failed (${response.status})`);
    return response.blob();
  }
}
