<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { env } from '$env/dynamic/public';
  import { KeyRound, Laptop, ShieldCheck, Trash2 } from '@lucide/svelte';
  import {
    ApiClient,
    ApiError,
    browserSession,
    type AccountSession,
    type PasskeySummary,
    type User
  } from '$lib/api';
  import { createPasskey, getPasskey, passkeyErrorMessage, passkeysSupported } from '$lib/webauthn';
  import { describeUserAgent, formatDate, formatDateTime, suggestedPasskeyName } from '$lib/account';
  import ConsoleBar from '$lib/components/account/ConsoleBar.svelte';

  const api = new ApiClient(env.PUBLIC_API_BASE_URL || '/api', fetch, false);
  api.onUnauthorized = () => {
    browserSession.clear();
    void goto('/login');
  };

  let account = $state<User | null>(null);
  let passkeys = $state<PasskeySummary[]>([]);
  let sessions = $state<AccountSession[]>([]);
  let supported = $state(true);
  let busy = $state(false);
  let error = $state('');
  let notice = $state('');
  let passkeyName = $state('Passkey');
  let password = $state('');

  const mfa = $derived(account?.mfaVerified ?? false);
  // Adding a first passkey needs the password. Adding or removing one once any exist needs
  // a passkey check, so a stolen password can never swap in someone else's passkey.
  const needsStepUp = $derived(passkeys.length > 0 && !mfa);

  onMount(() => {
    api.useSession(browserSession);
    if (!api.accessToken) return void goto('/login');
    supported = passkeysSupported();
    passkeyName = suggestedPasskeyName(navigator.userAgent);
    void load();
  });

  async function load() {
    try {
      [account, passkeys, sessions] = await Promise.all([api.me(), api.passkeys(), api.accountSessions()]);
    } catch (reason) {
      error = reason instanceof Error ? reason.message : 'Could not load your account.';
    }
  }

  async function run(task: () => Promise<void>, success: string) {
    busy = true;
    error = '';
    notice = '';
    try {
      await task();
      notice = success;
      await load();
    } catch (reason) {
      error =
        reason instanceof ApiError
          ? reason.code === 'mfa_required'
            ? 'Verify with one of your passkeys first.'
            : reason.status === 403
              ? 'Your current password is incorrect.'
              : reason.message
          : passkeyErrorMessage(reason);
    } finally {
      busy = false;
    }
  }

  const verify = () =>
    run(async () => {
      const { challengeId, options } = await api.startStepUp();
      await api.finishStepUp(challengeId, await getPasskey(options));
    }, 'Passkey verified. This session can manage passkeys and admin tools for the next 12 hours.');

  const addPasskey = () =>
    run(async () => {
      const { challengeId, options } = await api.startPasskeyRegistration(passkeys.length ? undefined : password);
      const credential = await createPasskey(options);
      await api.finishPasskeyRegistration(challengeId, passkeyName.trim() || 'Passkey', credential);
      password = '';
    }, 'Passkey added. You can now sign in without a password.');

  const removePasskey = (key: PasskeySummary) => {
    if (!confirm(`Remove “${key.name}”? You will no longer be able to sign in with it.`)) return;
    return run(() => api.deletePasskey(key.id), `Removed “${key.name}”.`);
  };

  const signOut = (session: AccountSession) =>
    run(() => api.revokeAccountSession(session.id), `Signed out ${describeUserAgent(session.userAgent)}.`);

  const signOutOthers = () =>
    run(async () => {
      await Promise.all(sessions.filter((session) => !session.current).map((session) => api.revokeAccountSession(session.id)));
    }, 'Signed out every other device.');
</script>

<svelte:head><title>Account security · Slimlytics</title></svelte:head>

<div class="console">
  <ConsoleBar current="account" isAdmin={account?.isAdmin} />
  <main id="main" class="console-main">
    <section class="console-head">
      <p class="eyebrow">Account</p>
      <h1>Account security</h1>
      <p>
        {account?.email ?? 'Loading…'} · Each device stays signed in for 30 days after you last use it.
      </p>
    </section>

    {#if error}<div class="alert" role="alert">{error}</div>{/if}
    {#if notice}<div class="console-notice" role="status">{notice}</div>{/if}

    <section class="console-card" aria-labelledby="passkeys-title">
      <div class="console-card-head">
        <div>
          <h2 id="passkeys-title">Passkeys</h2>
          <p class="card-intro">
            Sign in with Face ID, Touch ID, Windows Hello, or a security key instead of a password. Passkeys
            can't be phished, and admin tools require one.
          </p>
        </div>
        {#if mfa}<span class="chip good"><ShieldCheck size={13} aria-hidden="true" />Verified this session</span>{/if}
      </div>

      {#if passkeys.length}
        <ul class="console-list" aria-label="Your passkeys">
          {#each passkeys as key (key.id)}
            <li>
              <span class="console-icon"><KeyRound size={17} aria-hidden="true" /></span>
              <span class="grow">
                <strong>{key.name}</strong>
                <small>Added {formatDate(key.createdAt)} · Last used {formatDate(key.lastUsedAt)}</small>
              </span>
              <button class="danger" disabled={busy || !mfa} onclick={() => void removePasskey(key)}>
                <Trash2 size={15} aria-hidden="true" />Remove
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="console-empty">No passkeys yet.</p>
      {/if}

      {#if !supported}
        <p class="console-empty">This browser doesn't support passkeys.</p>
      {:else if needsStepUp}
        <div class="console-form step-up">
          <p class="console-empty">To add or remove a passkey, first verify with one you already have.</p>
          <button class="primary" disabled={busy} onclick={() => void verify()}>
            <ShieldCheck size={16} aria-hidden="true" />Verify with passkey
          </button>
        </div>
      {:else}
        <form
          class="console-form"
          onsubmit={(event) => {
            event.preventDefault();
            void addPasskey();
          }}
        >
          <label for="passkey-name">Passkey name<input id="passkey-name" type="text" bind:value={passkeyName} maxlength="100" required /></label>
          {#if !passkeys.length}
            <label for="current-password"
              >Current password<input id="current-password" type="password" bind:value={password} autocomplete="current-password" required /></label
            >
          {/if}
          <button class="primary" disabled={busy}><KeyRound size={16} aria-hidden="true" />Add a passkey</button>
        </form>
      {/if}
      {#if passkeys.length}
        <p class="console-empty">
          Lost every passkey? Sign in with your password and ask the server operator to run
          <code>scripts/passkey-reset.sh</code> for your account.
        </p>
      {/if}
    </section>

    <section class="console-card" aria-labelledby="devices-title">
      <div class="console-card-head">
        <div>
          <h2 id="devices-title">Signed-in devices</h2>
          <p class="card-intro">Sign out any device you don't recognise. Its access ends immediately.</p>
        </div>
        {#if sessions.length > 1}
          <button class="secondary" disabled={busy} onclick={() => void signOutOthers()}>Sign out other devices</button>
        {/if}
      </div>
      <ul class="console-list" aria-label="Signed-in devices">
        {#each sessions as session (session.id)}
          <li>
            <span class="console-icon"><Laptop size={17} aria-hidden="true" /></span>
            <span class="grow">
              <strong>{describeUserAgent(session.userAgent)}</strong>
              <small>
                Signed in {formatDate(session.createdAt)} with {session.authMethod === 'passkey' ? 'a passkey' : 'a password'} ·
                Active {formatDateTime(session.lastUsedAt)}
              </small>
            </span>
            {#if session.current}
              <span class="chip good">This device</span>
            {:else}
              <button class="secondary" disabled={busy} onclick={() => void signOut(session)}>Sign out</button>
            {/if}
          </li>
        {/each}
      </ul>
    </section>
  </main>
</div>
