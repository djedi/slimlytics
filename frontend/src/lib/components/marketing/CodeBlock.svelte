<script lang="ts">
  import { Copy } from '@lucide/svelte';

  // Copying is handled by the delegated [data-copy] listener in app.html so the block works on
  // pages that ship no Svelte client code.
  let { code, label, lang = 'sh' }: { code: string; label: string; lang?: string } = $props();
</script>

<figure class="code-block">
  <figcaption>
    <span>{lang}</span>
    <button type="button" data-copy={label} aria-label={`Copy ${label}`}>
      <Copy size={15} aria-hidden="true" /> <span data-copy-label>Copy</span>
    </button>
  </figcaption>
  <pre><code>{code}</code></pre>
  <span class="sr-only" aria-live="polite"></span>
</figure>

<style>
  .code-block {
    margin: 0 0 20px;
    border-radius: 14px;
    overflow: hidden;
    background: var(--m-ink-2, #0e1622);
    border: 1px solid rgba(255, 255, 255, 0.07);
  }
  figcaption {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 6px 4px 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
    color: #a3b0c2;
    font: 600 12px var(--m-mono, monospace);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 36px;
    padding: 0 12px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: #dbe4ee;
    font: 600 13px system-ui, sans-serif;
    letter-spacing: 0;
    text-transform: none;
    cursor: pointer;
  }
  button:hover {
    background: rgba(255, 255, 255, 0.08);
  }
  pre {
    margin: 0;
    padding: 16px 18px;
    overflow-x: auto;
    color: #dbe4ee;
    font: 14px / 1.7 var(--m-mono, monospace);
  }
  code {
    font: inherit;
  }
</style>
