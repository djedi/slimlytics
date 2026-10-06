import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { applyIgnoreToggle, createTracker, isIgnored, redactUrl, toCollectInput, trackerOptionsFromScript, IGNORE_STORAGE_KEY } from '../src/index';

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

describe('privacy', () => {
  beforeEach(() => {
    Object.defineProperty(navigator, 'doNotTrack', { value: '0', configurable: true });
    Object.defineProperty(navigator, 'globalPrivacyControl', { value: false, configurable: true });
  });

  it('redacts sensitive query values and preserves safe parameters', () => {
    expect(redactUrl('https://app.test/a?utm_source=email&token=secret&email=a%40b.com#x'))
      .toBe('https://app.test/a?utm_source=email&token=%5BREDACTED%5D&email=%5BREDACTED%5D');
  });

  it('does not track when DNT is enabled', async () => {
    Object.defineProperty(navigator, 'doNotTrack', { value: '1', configurable: true });
    const send = vi.fn();
    const tracker = createTracker({ writeKey: 'key', endpoint: '/api/collect', transport: send });
    tracker.page();
    await tracker.flush();
    expect(send).not.toHaveBeenCalled();
  });

  it('sends a privacy-reduced event when GPC is enabled', async () => {
    Object.defineProperty(navigator, 'globalPrivacyControl', { value: true, configurable: true });
    const send = vi.fn().mockResolvedValue(true);
    const tracker = createTracker({
      writeKey: 'key',
      endpoint: '/api/collect',
      transport: send,
      autoTrack: false
    });
    tracker.event('signup', { email: 'private@example.com', plan: 'pro' });
    await tracker.flush();

    expect(send).toHaveBeenCalledOnce();
    expect(send.mock.calls[0][1].events[0]).toMatchObject({ privacyControl: 'gpc' });
    expect(toCollectInput(send.mock.calls[0][1].events[0])).toMatchObject({
      name: 'signup',
      privacyControl: 'gpc',
      properties: {}
    });
  });

  it('can explicitly deny collection under GPC', async () => {
    Object.defineProperty(navigator, 'globalPrivacyControl', { value: true, configurable: true });
    const send = vi.fn();
    const tracker = createTracker({
      writeKey: 'key',
      transport: send,
      autoTrack: false,
      gpcMode: 'deny'
    });
    tracker.page();
    await tracker.flush();
    expect(send).not.toHaveBeenCalled();
  });
});

describe('tracker', () => {
  beforeEach(() => {
    Object.defineProperty(navigator, 'doNotTrack', { value: '0', configurable: true });
    Object.defineProperty(navigator, 'globalPrivacyControl', { value: false, configurable: true });
    history.replaceState({}, '', '/start?token=nope&utm_source=test');
  });
  afterEach(() => vi.useRealTimers());

  it('is cookieless, batches unique page and custom events, and supports consent', async () => {
    const send = vi.fn().mockResolvedValue(true);
    const tracker = createTracker({ writeKey: 'wk_1', endpoint: '/collect', transport: send, autoTrack: false });
    tracker.page();
    tracker.page();
    tracker.event('signup', { plan: 'pro' });
    await tracker.flush();
    const [url, payload] = send.mock.calls[0];
    expect(url).toBe('/collect/wk_1');
    expect(payload.events).toHaveLength(3);
    expect(new Set(payload.events.map((event: { id: string }) => event.id)).size).toBe(3);
    expect(payload.events[0].url).not.toContain('nope');
    expect(document.cookie).toBe('');
    tracker.consent(false);
    tracker.event('blocked');
    await tracker.flush();
    expect(send).toHaveBeenCalledTimes(1);
  });

  it('tracks SPA navigation, downloads and outbound links without form values', async () => {
    const send = vi.fn().mockResolvedValue(true);
    const tracker = createTracker({ writeKey: 'wk', transport: send, autoTrack: true, batchInterval: 10_000 });
    // Auto-track flushes the first pageview immediately.
    await tick();
    expect(send).toHaveBeenCalled();
    history.pushState({}, '', '/next');
    await tick();
    const download = document.createElement('a');
    download.href = '/report.pdf';
    download.textContent = 'report';
    document.body.append(download);
    download.addEventListener('click', (click) => click.preventDefault());
    download.click();
    const outbound = document.createElement('a');
    outbound.href = 'https://outside.test/path';
    document.body.append(outbound);
    outbound.addEventListener('click', (click) => click.preventDefault());
    outbound.click();
    const form = document.createElement('form');
    form.innerHTML = '<input value="private">';
    document.body.append(form);
    form.dispatchEvent(new Event('submit', { bubbles: true }));
    await tracker.flush();
    const events = send.mock.calls.flatMap((call) => call[1].events as Array<{ type: string; name?: string }>);
    expect(events.some((event) => event.type === 'page')).toBe(true);
    expect(events.some((event) => event.name === 'download')).toBe(true);
    expect(events.some((event) => event.name === 'outbound')).toBe(true);
    expect(JSON.stringify(events)).not.toContain('private');
    tracker.destroy();
  });

  it('bootstraps options from installation script attributes', () => {
    const script = document.createElement('script');
    script.dataset.writeKey = 'wk_anti';
    script.dataset.endpoint = 'https://analytics.example/api/e';
    script.dataset.autoTrack = 'false';
    script.dataset.respectDnt = 'true';
    script.dataset.consent = 'denied';

    expect(trackerOptionsFromScript(script)).toEqual({
      writeKey: 'wk_anti',
      endpoint: 'https://analytics.example/api/e',
      autoTrack: false,
      respectDnt: true,
      gpcMode: 'reduce',
      consent: 'denied'
    });
    expect(trackerOptionsFromScript(document.createElement('script'))).toBeUndefined();
  });

  it('can send to an exact first-party beacon path without appending the write key', async () => {
    const urls: string[] = [];
    const tracker = createTracker({
      writeKey: 'site-key',
      endpoint: '/0d31360a3101',
      appendWriteKey: false,
      autoTrack: false,
      transport: async (url) => { urls.push(url); return true; }
    });
    tracker.page();
    await tracker.flush();
    expect(urls).toEqual(['/0d31360a3101']);
    tracker.destroy();
  });
});

describe('default transport', () => {
  it('maps queued events to the collector API contract', () => {
    const mapped = toCollectInput({
      id: 'event-1',
      type: 'event',
      name: 'signup',
      timestamp: '2026-07-29T00:00:00Z',
      url: 'https://example.com/thanks',
      properties: { plan: 'pro' }
    });
    expect(mapped).toMatchObject({
      name: 'signup',
      occurredAt: '2026-07-29T00:00:00Z',
      url: 'https://example.com/thanks',
      properties: { plan: 'pro' }
    });
  });

  it('prefers fetch with JSON content-type over sendBeacon', async () => {
    const beacon = vi.fn().mockReturnValue(true);
    Object.defineProperty(navigator, 'sendBeacon', { value: beacon, configurable: true });
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(null, { status: 202 }));
    const tracker = createTracker({ writeKey: 'wk', autoTrack: false });
    tracker.event('ping');
    await tracker.flush();
    expect(fetchSpy).toHaveBeenCalledOnce();
    expect(fetchSpy.mock.calls[0][1]).toMatchObject({
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      keepalive: true,
      credentials: 'omit'
    });
    expect(beacon).not.toHaveBeenCalled();
  });

  it('omits blank referrers from the collector payload', () => {
    const mapped = toCollectInput({
      id: 'event-2',
      type: 'page',
      timestamp: '2026-07-29T00:00:00Z',
      url: 'https://example.com/',
      referrer: '   '
    });
    expect(mapped.referrer).toBeUndefined();
  });
});

describe('ignore my visits', () => {
  beforeEach(() => {
    // Node 25+ ships an empty global localStorage that shadows jsdom's; use the real one.
    const storage = (globalThis as unknown as { jsdom: { window: Window } }).jsdom.window.localStorage;
    Object.defineProperty(window, 'localStorage', { value: storage, configurable: true });
    window.localStorage.clear();
    history.replaceState(null, '', '/');
  });
  afterEach(() => {
    window.localStorage.clear();
    history.replaceState(null, '', '/');
  });

  it('turns ignore on and off from the URL fragment and strips the marker', () => {
    history.replaceState(null, '', '/pricing?plan=pro#slimlytics-ignore');
    expect(applyIgnoreToggle()).toBe(true);
    expect(window.localStorage.getItem(IGNORE_STORAGE_KEY)).toBe('true');
    expect(location.hash).toBe('');
    expect(location.pathname + location.search).toBe('/pricing?plan=pro');

    history.replaceState(null, '', '/#slimlytics-ignore=off');
    expect(applyIgnoreToggle()).toBe(false);
    expect(window.localStorage.getItem(IGNORE_STORAGE_KEY)).toBeNull();
  });

  it('applies the fragment even when autoTrack is disabled', async () => {
    history.replaceState(null, '', '/#slimlytics-ignore');
    const send = vi.fn().mockResolvedValue(true);
    const tracker = createTracker({ writeKey: 'key', transport: send, autoTrack: false });
    expect(window.localStorage.getItem(IGNORE_STORAGE_KEY)).toBe('true');
    expect(tracker.page()).toBeUndefined();
    await tracker.flush();
    expect(send).not.toHaveBeenCalled();
    tracker.destroy();
  });

  it('honors the opt-out in memory when localStorage cannot be written', () => {
    const setItem = vi.spyOn(window.localStorage, 'setItem').mockImplementation(() => { throw new Error('quota'); });
    const removeItem = vi.spyOn(window.localStorage, 'removeItem').mockImplementation(() => { throw new Error('blocked'); });
    history.replaceState(null, '', '/#slimlytics-ignore');
    expect(applyIgnoreToggle()).toBe(true);
    expect(isIgnored()).toBe(true);
    history.replaceState(null, '', '/#slimlytics-ignore=off');
    expect(applyIgnoreToggle()).toBe(false);
    setItem.mockRestore();
    removeItem.mockRestore();
    history.replaceState(null, '', '/#slimlytics-ignore=off');
    applyIgnoreToggle();
  });

  it('applies the toggle on same-document hash navigation', () => {
    const tracker = createTracker({ writeKey: 'key', transport: vi.fn().mockResolvedValue(true), autoTrack: false });
    history.replaceState(null, '', '/#slimlytics-ignore');
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    expect(isIgnored()).toBe(true);
    expect(location.hash).toBe('');
    history.replaceState(null, '', '/#slimlytics-ignore=off');
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    expect(isIgnored()).toBe(false);
    tracker.destroy();
  });

  it('discards events queued before an opt-out so resuming cannot replay them', async () => {
    const send = vi.fn().mockResolvedValue(true);
    const tracker = createTracker({ writeKey: 'key', transport: send, autoTrack: false, batchSize: 50 });
    tracker.event('before-opt-out');
    window.localStorage.setItem(IGNORE_STORAGE_KEY, 'true');
    await tracker.flush();
    window.localStorage.removeItem(IGNORE_STORAGE_KEY);
    await tracker.flush();
    expect(send).not.toHaveBeenCalled();
    tracker.destroy();
  });

  it('leaves unrelated fragments alone', () => {
    history.replaceState(null, '', '/docs#install');
    expect(applyIgnoreToggle()).toBe(false);
    expect(location.hash).toBe('#install');
  });

  it('does not send anything while the browser is ignored', async () => {
    window.localStorage.setItem(IGNORE_STORAGE_KEY, 'true');
    expect(isIgnored()).toBe(true);
    const send = vi.fn().mockResolvedValue(true);
    const tracker = createTracker({ writeKey: 'key', transport: send, autoTrack: false });
    expect(tracker.page()).toBeUndefined();
    expect(tracker.event('signup')).toBeUndefined();
    await tracker.flush();
    expect(send).not.toHaveBeenCalled();
    tracker.destroy();
  });

  it('tracks normally when storage is unavailable', () => {
    const spy = vi.spyOn(window.localStorage, 'getItem').mockImplementation(() => { throw new Error('blocked'); });
    expect(isIgnored()).toBe(false);
    spy.mockRestore();
  });
});
