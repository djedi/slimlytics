<script module lang="ts">
  import {
    siAndroid,
    siApple,
    siBlackberry,
    siBrave,
    siDuckduckgo,
    siFirefoxbrowser,
    siFreebsd,
    siGooglechrome,
    siIos,
    siLinux,
    siOpera,
    siSafari,
    siSamsung,
    siUbuntu,
    siVivaldi,
    type SimpleIcon
  } from 'simple-icons';

  export type IconDimension = 'countries' | 'devices' | 'browsers' | 'operating-systems';

  // Matched in order against the lowercased label; first hit wins. Microsoft doesn't license
  // its marks to simple-icons, so Edge, IE and Windows fall through to a generic icon.
  const browserLogos: [string, SimpleIcon][] = [
    ['samsung', siSamsung],
    ['opera', siOpera],
    ['vivaldi', siVivaldi],
    ['brave', siBrave],
    ['duckduckgo', siDuckduckgo],
    ['firefox', siFirefoxbrowser],
    ['chrom', siGooglechrome],
    ['safari', siSafari]
  ];
  const osLogos: [string, SimpleIcon][] = [
    ['android', siAndroid],
    ['iphone', siIos],
    ['ipad', siIos],
    ['ipod', siIos],
    ['ios', siIos],
    ['mac', siApple],
    ['ubuntu', siUbuntu],
    ['chromeos', siGooglechrome],
    ['linux', siLinux],
    ['freebsd', siFreebsd],
    ['blackberry', siBlackberry]
  ];

  export function brandLogo(dimension: IconDimension, label: string): SimpleIcon | undefined {
    const list = dimension === 'browsers' ? browserLogos : dimension === 'operating-systems' ? osLogos : [];
    const name = label.toLowerCase();
    return list.find(([match]) => name.includes(match))?.[1];
  }
</script>

<script lang="ts">
  import { AppWindow, Globe, Monitor, Smartphone, Tablet } from '@lucide/svelte';
  import { flagEmoji } from '../ui';

  let { dimension, label }: { dimension: IconDimension; label: string } = $props();

  const logo = $derived(brandLogo(dimension, label));
  const device = $derived(label.toLowerCase());
  // Near-black marks (Apple, iOS) would vanish in dark mode, so they follow the text color.
  const fill = $derived.by(() => {
    if (!logo) return undefined;
    const [r, g, b] = [0, 2, 4].map((at) => parseInt(logo.hex.slice(at, at + 2), 16));
    return 0.299 * r + 0.587 * g + 0.114 * b < 60 ? 'currentColor' : `#${logo.hex}`;
  });
</script>

<span class="dimension-icon" aria-hidden="true">
  {#if dimension === 'countries'}
    <span class="flag">{flagEmoji(label)}</span>
  {:else if dimension === 'devices'}
    {#if device === 'mobile'}<Smartphone size={15} />{:else if device === 'tablet'}<Tablet
        size={15}
      />{:else if device === 'desktop'}<Monitor size={15} />{:else}<Globe size={15} />{/if}
  {:else if logo}
    <svg viewBox="0 0 24 24" width="15" height="15" class="logo" {fill}><path d={logo.path} /></svg>
  {:else if dimension === 'operating-systems' && label.toLowerCase().includes('windows')}
    <AppWindow size={15} />
  {:else}
    <Globe size={15} />
  {/if}
</span>

<style>
  .dimension-icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    margin-left: 8px;
    color: var(--muted);
  }
  .logo {
    color: var(--text);
  }
  .flag {
    font-size: 15px;
    line-height: 1;
  }
</style>
