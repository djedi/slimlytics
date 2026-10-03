<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { env } from '$env/dynamic/public';
  import { KeyRound, LogOut, Search, ShieldAlert, ShieldCheck, Trash2, UserCheck, UserX, X } from '@lucide/svelte';
  import {
    ApiClient,
    ApiError,
    browserSession,
    type AdminAuditEntry,
    type AdminOverview,
    type AdminUser,
    type AdminUserDetail,
    type User
  } from '$lib/api';
  import { getPasskey, passkeyErrorMessage } from '$lib/webauthn';
  import { auditLabel, describeUserAgent, formatDate, formatDateTime } from '$lib/account';
  import ConsoleBar from '$lib/components/account/ConsoleBar.svelte';

  const PAGE_SIZE = 50;
  const api = new ApiClient(env.PUBLIC_API_BASE_URL || '/api', fetch, false);
  api.onUnauthorized = () => {
    browserSession.clear();
    void goto('/login');
  };

  type Gate = 'loading' | 'forbidden' | 'needs-passkey' | 'needs-verification' | 'ready';
  let gate = $state<Gate>('loading');
  let account = $state<User | null>(null);
  let overview = $state<AdminOverview | null>(null);
  let users = $state<AdminUser[]>([]);
  let total = $state(0);
  let offset = $state(0);
  let query = $state('');
  let audit = $state<AdminAuditEntry[]>([]);
  let detail = $state<AdminUserDetail | null>(null);
  let confirmEmail = $state('');
  let busy = $state(false);
  let error = $state('');
  let notice = $state('');
  let searchTimer: ReturnType<typeof setTimeout> | undefined;

  onMount(() => {
    api.useSession(browserSession);
    if (!api.accessToken) return void goto('/login');
    void start();
  });

  async function start() {
    try {
      account = await api.me();
    } catch {
      return;
    }
    if (!account.isAdmin) gate = 'forbidden';
    else if (!account.mfaVerified) gate = account.passkeyCount ? 'needs-verification' : 'needs-passkey';
    else await loadAll();
  }

  /** Runs an admin request; an expired passkey check sends the admin back to the gate. */
  async function guarded<T>(task: () => Promise<T>): Promise<T | undefined> {
    try {
      return await task();
    } catch (reason) {
      if (reason instanceof ApiError && reason.code === 'mfa_required') {
        gate = 'needs-verification';
        detail = null;
      } else error = reason instanceof Error ? reason.message : 'Request failed.';
      return undefined;
    }
  }

  async function loadAll() {
    const result = await guarded(() => Promise.all([api.adminOverview(), api.adminUsers(query, offset, PAGE_SIZE), api.adminAudit(50)]));
    if (!result) return;
    [overview, { users, total }, audit] = [result[0], result[1], result[2]];
    gate = 'ready';
  }

  async function loadUsers() {
    const page = await guarded(() => api.adminUsers(query, offset, PAGE_SIZE));
    if (page) ({ users, total } = page);
  }

  function onSearch() {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      offset = 0;
      void loadUsers();
    }, 250);
  }

  async function verify() {
    busy = true;
    error = '';
    try {
      const { challengeId, options } = await api.startStepUp();
      await api.finishStepUp(challengeId, await getPasskey(options));
      await loadAll();
    } catch (reason) {
      error = reason instanceof ApiError ? reason.message : passkeyErrorMessage(reason);
    } finally {
      busy = false;
    }
  }

  async function open(user: AdminUser) {
    confirmEmail = '';
    notice = '';
    const result = await guarded(() => api.adminUser(user.id));
    if (result) detail = result;
  }

  async function act(label: string, task: () => Promise<void>, closeAfter = false) {
    if (!detail) return;
    busy = true;
    error = '';
    notice = '';
    const id = detail.user.id;
    const done = await guarded(async () => {
      await task();
      return true;
    });
    busy = false;
    if (!done) return;
    notice = label;
    if (closeAfter) detail = null;
    else {
      const refreshed = await guarded(() => api.adminUser(id));
      if (refreshed) detail = refreshed;
    }
    await loadAll();
  }

  const disable = () =>
    detail &&
    confirm(`Disable ${detail.user.email}? They are signed out everywhere and cannot sign in until re-enabled.`) &&
    act(`Disabled ${detail.user.email}.`, () => api.adminUserAction(detail!.user.id, 'disable'));
  const enable = () => detail && act(`Re-enabled ${detail.user.email}.`, () => api.adminUserAction(detail!.user.id, 'enable'));
  const revoke = () =>
    detail && act(`Signed ${detail.user.email} out everywhere.`, () => api.adminUserAction(detail!.user.id, 'revoke-sessions'));
  const remove = () => {
    if (!detail) return;
    const email = detail.user.email;
    return act(`Deleted ${email}.`, () => api.adminDeleteUser(detail!.user.id, confirmEmail), true);
  };

  const stats = $derived(
    overview
      ? [
          { label: 'Accounts', value: overview.users },
          { label: 'Active in 7 days', value: overview.activeUsersLast7Days },
          { label: 'New in 7 days', value: overview.signupsLast7Days },
          { label: 'Sites', value: overview.sites },
          { label: 'Disabled', value: overview.disabledUsers },
          { label: 'Admins', value: overview.admins }
        ]
      : []
  );
</script>

<svelte:head><title>Admin · Slimlytics</title></svelte:head>

<div class="console">
  <ConsoleBar current="admin" isAdmin={account?.isAdmin} />
  <main id="main" class="console-main">
    {#if gate === 'loading'}
      <div class="loading" role="status"><span></span><p>Loading…</p></div>
    {:else if gate === 'forbidden'}
      <section class="console-card console-gate">
        <span class="console-icon"><ShieldAlert size={24} aria-hidden="true" /></span>
        <h1>No admin access</h1>
        <p>This account isn't an administrator. Ask the person who runs this Slimlytics server for access.</p>
        <a class="secondary" href="/app">Back to dashboard</a>
      </section>
    {:else if gate === 'needs-passkey'}
      <section class="console-card console-gate">
        <span class="console-icon"><KeyRound size={24} aria-hidden="true" /></span>
        <h1>Add a passkey to continue</h1>
        <p>Admin tools require two-factor sign-in with a passkey. Add one to your account, then come back here.</p>
        <a class="primary" href="/account">Set up a passkey</a>
      </section>
    {:else if gate === 'needs-verification'}
      <section class="console-card console-gate">
        <span class="console-icon"><ShieldCheck size={24} aria-hidden="true" /></span>
        <h1>Verify it's you</h1>
        <p>Admin tools need a passkey check. It lasts 12 hours on this device.</p>
        {#if error}<div class="alert" role="alert">{error}</div>{/if}
        <button class="primary" disabled={busy} onclick={() => void verify()}>
          <KeyRound size={16} aria-hidden="true" />{busy ? 'Waiting for passkey…' : 'Verify with passkey'}
        </button>
      </section>
    {:else}
      <section class="console-head">
        <p class="eyebrow">Administration</p>
        <h1>Accounts</h1>
        <p>Every action here is recorded in the audit log below.</p>
      </section>

      {#if error}<div class="alert" role="alert">{error}</div>{/if}
      {#if notice && !detail}<div class="console-notice" role="status">{notice}</div>{/if}

      <section class="stat-row" aria-label="Platform totals">
        {#each stats as stat (stat.label)}
          <div class="stat"><span>{stat.label}</span><strong>{stat.value.toLocaleString()}</strong></div>
        {/each}
      </section>

      <section class="console-card" aria-labelledby="users-title">
        <div class="console-card-head">
          <h2 id="users-title">Users <span class="muted">({total.toLocaleString()})</span></h2>
          <label class="user-search" for="user-search">
            <Search size={16} aria-hidden="true" /><span class="sr-only">Search by email</span>
            <input id="user-search" type="search" placeholder="Search by email" bind:value={query} oninput={onSearch} maxlength="200" />
          </label>
        </div>
        <div class="table-scroll">
          <table class="user-table">
            <thead>
              <tr>
                <th scope="col">Email</th><th scope="col">Joined</th><th scope="col">Last sign-in</th>
                <th scope="col" class="num">Sites</th><th scope="col" class="num">Passkeys</th><th scope="col" class="num">Devices</th>
                <th scope="col">Plan</th>
              </tr>
            </thead>
            <tbody>
              {#each users as user (user.id)}
                <tr class:selected={detail?.user.id === user.id}>
                  <td>
                    <button class="user-link" onclick={() => void open(user)}>{user.email}</button>
                    {#if user.isAdmin}<span class="chip info">Admin</span>{/if}
                    {#if user.disabledAt}<span class="chip bad">Disabled</span>{/if}
                  </td>
                  <td>{formatDate(user.createdAt)}</td>
                  <td>{formatDate(user.lastLoginAt)}</td>
                  <td class="num">{user.siteCount}</td>
                  <td class="num">{user.passkeyCount}</td>
                  <td class="num">{user.activeSessions}</td>
                  <td>{user.plan ?? '—'}</td>
                </tr>
              {:else}
                <tr><td colspan="7" class="console-empty">No accounts match “{query}”.</td></tr>
              {/each}
            </tbody>
          </table>
        </div>
        {#if total > PAGE_SIZE}
          <div class="pager">
            <button class="secondary" disabled={offset === 0} onclick={() => { offset = Math.max(0, offset - PAGE_SIZE); void loadUsers(); }}>Previous</button>
            <span class="muted">{offset + 1}–{Math.min(offset + PAGE_SIZE, total)} of {total}</span>
            <button class="secondary" disabled={offset + PAGE_SIZE >= total} onclick={() => { offset += PAGE_SIZE; void loadUsers(); }}>Next</button>
          </div>
        {/if}
      </section>

      <section class="console-card" aria-labelledby="audit-title">
        <h2 id="audit-title">Audit log</h2>
        {#if audit.length}
          <ul class="console-list">
            {#each audit as entry (entry.id)}
              <li>
                <span class="grow">
                  <strong>{auditLabel(entry.action)} · {entry.targetEmail ?? 'unknown account'}</strong>
                  <small>by {entry.actorEmail} · {formatDateTime(entry.createdAt)}</small>
                </span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="console-empty">No admin actions yet.</p>
        {/if}
      </section>
    {/if}
  </main>

  {#if detail}
    <button class="drawer-backdrop" aria-label="Close account details" onclick={() => (detail = null)}></button>
    <aside class="user-drawer" aria-labelledby="detail-title">
      <div class="console-card-head">
        <div>
          <p class="eyebrow">Account</p>
          <h2 id="detail-title">{detail.user.email}</h2>
          <p class="card-intro">
            Joined {formatDate(detail.user.createdAt)} · Last sign-in {formatDate(detail.user.lastLoginAt)}
            {#if detail.user.plan}· Plan {detail.user.plan}{/if}
            {#if detail.user.subscriptionStatus}({detail.user.subscriptionStatus}){/if}
          </p>
        </div>
        <button class="icon-button" aria-label="Close" onclick={() => (detail = null)}><X /></button>
      </div>
      <div class="chips">
        {#if detail.user.isAdmin}<span class="chip info">Admin</span>{/if}
        {#if detail.user.disabledAt}<span class="chip bad">Disabled {formatDate(detail.user.disabledAt)}</span>{:else}<span class="chip good">Active</span>{/if}
        <span class="chip">{detail.activeApiTokens} API tokens</span>
      </div>
      {#if notice}<div class="console-notice" role="status">{notice}</div>{/if}
      {#if error}<div class="alert" role="alert">{error}</div>{/if}

      <h3>Sites ({detail.sites.length})</h3>
      {#if detail.sites.length}
        <ul class="console-list">
          {#each detail.sites as site (site.id)}
            <li><span class="grow"><strong>{site.name}</strong><small>{site.domain}</small></span><span class="chip">{site.role}</span></li>
          {/each}
        </ul>
      {:else}<p class="console-empty">No sites.</p>{/if}

      <h3>Signed-in devices ({detail.sessions.length})</h3>
      {#if detail.sessions.length}
        <ul class="console-list">
          {#each detail.sessions as session (session.id)}
            <li>
              <span class="grow">
                <strong>{describeUserAgent(session.userAgent)}</strong>
                <small>{session.authMethod} · active {formatDateTime(session.lastUsedAt)}</small>
              </span>
            </li>
          {/each}
        </ul>
      {:else}<p class="console-empty">Not signed in anywhere.</p>{/if}

      <h3>Passkeys ({detail.passkeys.length})</h3>
      {#if detail.passkeys.length}
        <ul class="console-list">
          {#each detail.passkeys as key (key.id)}
            <li><span class="grow"><strong>{key.name}</strong><small>Last used {formatDate(key.lastUsedAt)}</small></span></li>
          {/each}
        </ul>
      {:else}<p class="console-empty">No passkeys.</p>{/if}

      {#if detail.user.isAdmin}
        <p class="console-empty">Admin accounts can't be changed here. Remove admin access with <code>scripts/admin-grant.sh</code> first.</p>
      {:else}
        <h3>Actions</h3>
        <div class="actions">
          <button class="secondary" disabled={busy} onclick={() => void revoke()}><LogOut size={15} aria-hidden="true" />Sign out everywhere</button>
          {#if detail.user.disabledAt}
            <button class="secondary" disabled={busy} onclick={() => void enable()}><UserCheck size={15} aria-hidden="true" />Re-enable account</button>
          {:else}
            <button class="danger" disabled={busy} onclick={() => void disable()}><UserX size={15} aria-hidden="true" />Disable account</button>
          {/if}
        </div>
        <form
          class="delete-zone"
          onsubmit={(event) => {
            event.preventDefault();
            void remove();
          }}
        >
          <strong>Delete account</strong>
          <p class="card-intro">
            Permanently deletes the account and every site only it owns, with all their analytics. This cannot be undone.
          </p>
          <label for="confirm-email"
            ><span>Type <b>{detail.user.email}</b> to confirm</span>
            <input id="confirm-email" type="email" bind:value={confirmEmail} autocomplete="off" />
          </label>
          <button class="danger" disabled={busy || confirmEmail.trim().toLowerCase() !== detail.user.email}>
            <Trash2 size={15} aria-hidden="true" />Delete account
          </button>
        </form>
      {/if}
    </aside>
  {/if}
</div>

<style>
  .stat-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 12px;
  }
  .stat {
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 14px 16px;
    display: grid;
    gap: 4px;
  }
  .stat span {
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
  }
  .stat strong {
    font: 800 24px/1.1 var(--display);
    font-variant-numeric: tabular-nums;
  }
  .user-search {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    min-width: min(320px, 100%);
  }
  .table-scroll {
    overflow-x: auto;
  }
  .user-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  .user-table th {
    text-align: left;
    color: var(--muted);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding: 8px 10px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  .user-table td {
    padding: 10px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  .user-table .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .user-table tr.selected td {
    background: color-mix(in srgb, var(--accent) 6%, transparent);
  }
  .user-link {
    border: 0;
    background: none;
    padding: 0;
    color: var(--text);
    font-weight: 650;
    cursor: pointer;
    margin-right: 6px;
  }
  .user-link:hover {
    color: var(--accent);
    text-decoration: underline;
  }
  .pager {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
  }
  .drawer-backdrop {
    position: fixed;
    inset: 0;
    border: 0;
    background: rgba(8, 12, 18, 0.45);
    z-index: 40;
    cursor: default;
  }
  .user-drawer {
    position: fixed;
    inset: 0 0 0 auto;
    width: min(520px, 100%);
    background: var(--surface);
    border-left: 1px solid var(--line);
    z-index: 41;
    overflow-y: auto;
    padding: 24px;
    display: grid;
    align-content: start;
    gap: 14px;
    box-shadow: var(--shadow);
  }
  .user-drawer h2 {
    font: 700 19px/1.3 var(--display);
    margin: 0;
    overflow-wrap: anywhere;
  }
  .user-drawer h3 {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    margin: 8px 0 0;
  }
  .chips,
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .delete-zone {
    display: grid;
    gap: 10px;
    padding: 16px;
    border: 1px solid color-mix(in srgb, var(--red) 35%, var(--line));
    border-radius: 10px;
    margin-top: 8px;
  }
  .delete-zone label {
    display: grid;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .delete-zone .danger {
    justify-self: start;
  }
  .console-gate h1 {
    font: 800 24px/1.2 var(--display);
    margin: 0;
  }
</style>
