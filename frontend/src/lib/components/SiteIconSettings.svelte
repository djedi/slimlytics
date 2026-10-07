<script lang="ts">
  import { RotateCcw } from '@lucide/svelte';
  import type { Site, SiteIconMode, SiteIconSettings } from '$lib/api';
  import SiteIcon from './SiteIcon.svelte';

  let {
    site,
    save,
    apiBase
  }: { site: Site; save: (settings: SiteIconSettings) => Promise<void>; apiBase?: string } = $props();

  type ColorKey = 'background' | 'backgroundEnd' | 'foreground';
  const colorFields: { key: ColorKey; label: string; fallback: string }[] = [
    { key: 'background', label: 'Background', fallback: '#12a97c' },
    { key: 'backgroundEnd', label: 'Gradient end', fallback: '#2386c8' },
    { key: 'foreground', label: 'Initials', fallback: '#ffffff' }
  ];

  let mode = $state<SiteIconMode>('initials');
  let colors = $state<Record<ColorKey, string>>({ background: '', backgroundEnd: '', foreground: '' });
  let saving = $state(false);
  let error = $state('');
  let saved = $state(false);

  // Reset the form whenever a different site (or a saved update) comes in.
  $effect(() => {
    mode = site.iconMode ?? 'initials';
    colors = {
      background: site.iconBackground ?? '',
      backgroundEnd: site.iconBackgroundEnd ?? '',
      foreground: site.iconForeground ?? ''
    };
  });

  const preview = $derived({
    ...site,
    iconMode: mode,
    iconBackground: colors.background || null,
    iconBackgroundEnd: colors.background ? colors.backgroundEnd || null : null,
    iconForeground: colors.foreground || null
  });

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    error = '';
    saved = false;
    try {
      await save({ mode, ...colors });
      saved = true;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Could not save the icon.';
    } finally {
      saving = false;
    }
  }
</script>

<form class="panel settings-card icon-settings" onsubmit={submit}>
  <p class="eyebrow">Site icon</p>
  <div class="icon-head">
    <SiteIcon site={preview} size={52} {apiBase} />
    <div>
      <h2>How {site.name} looks</h2>
      <p class="muted">Shown in the site picker and on the all-sites dashboard.</p>
    </div>
  </div>

  <div class="theme-options icon-modes" role="radiogroup" aria-label="Icon style">
    <button type="button" role="radio" aria-checked={mode === 'initials'} class:active={mode === 'initials'} onclick={() => (mode = 'initials')}>
      <span class="mode-glyph">{site.name.slice(0, 2).toUpperCase()}</span><span>Initials</span>
    </button>
    <button type="button" role="radio" aria-checked={mode === 'favicon'} class:active={mode === 'favicon'} onclick={() => (mode = 'favicon')}>
      <span class="mode-glyph">★</span><span>Website favicon</span>
    </button>
  </div>
  {#if mode === 'favicon'}
    <p class="field-help">
      Saving fetches the icon from <code>https://{site.domain}/</code>. If none can be loaded, the icon stays as it was.
    </p>
  {/if}

  <div class="color-fields">
    {#each colorFields as field}
      {@const disabled = field.key === 'backgroundEnd' && !colors.background}
      <label class:auto={!colors[field.key]}>
        <span>{field.label}</span>
        <span class="color-row">
          <input
            type="color"
            value={colors[field.key] || field.fallback}
            {disabled}
            oninput={(event) => (colors[field.key] = event.currentTarget.value)}
          />
          <small>{colors[field.key] || (field.key === 'backgroundEnd' ? 'Solid' : 'Auto')}</small>
          {#if colors[field.key]}
            <button type="button" class="reset" aria-label={`Reset ${field.label.toLowerCase()} color`} title="Reset" onclick={() => (colors[field.key] = '')}
              ><RotateCcw size={13} aria-hidden="true" /></button
            >
          {/if}
        </span>
      </label>
    {/each}
  </div>

  {#if error}<div class="alert" role="alert">{error}</div>{/if}
  {#if saved}<div class="success-message" role="status">Icon saved.</div>{/if}
  <button class="primary" type="submit" disabled={saving}>{saving ? (mode === 'favicon' ? 'Fetching favicon…' : 'Saving…') : 'Save icon'}</button>
</form>

<style>
  .icon-settings {
    display: grid;
    gap: 14px;
    align-content: start;
  }
  .icon-settings .eyebrow {
    margin: 0;
  }
  .icon-head {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .icon-head p {
    margin: 4px 0 0;
  }
  .icon-modes {
    grid-template-columns: repeat(2, 1fr);
    margin-top: 0;
  }
  .mode-glyph {
    font-weight: 800;
    font-size: 12px;
  }
  .field-help {
    margin: 0;
    color: var(--muted);
    font-size: 11px;
  }
  .color-fields {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }
  .color-fields label {
    display: grid;
    gap: 6px;
    font-size: 11px;
    font-weight: 700;
  }
  .color-row {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .color-row small {
    color: var(--muted);
    font-weight: 500;
    font-family: ui-monospace, monospace;
  }
  .auto input {
    opacity: 0.45;
  }
  input[type='color'] {
    width: 34px;
    height: 28px;
    padding: 2px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--surface);
    cursor: pointer;
  }
  input[type='color']:disabled {
    cursor: not-allowed;
  }
  .reset {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--muted);
    cursor: pointer;
  }
  .reset:hover {
    color: var(--accent);
  }
  .primary {
    justify-self: start;
  }
  @media (max-width: 520px) {
    .color-fields {
      grid-template-columns: 1fr;
    }
  }
</style>
