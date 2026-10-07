<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { goto, replaceState } from '$app/navigation';
  import { page } from '$app/state';
  import { env } from '$env/dynamic/public';
  import {
    Activity,
    ArrowRight,
    BarChart3,
    Bell,
    CalendarDays,
    ChevronDown,
    CircleDot,
    Compass,
    Download,
    Eye,
    FileText,
    Gauge,
    Goal as GoalIcon,
    Globe2,
    LayoutDashboard,
    LogOut,
    KeyRound,
    Menu,
    Monitor,
    Moon,
    Pause,
    Play,
    Layers,
    Plus,
    Radio,
    Search,
    Send,
    Settings,
    ShieldCheck,
    Timer,
    Smartphone,
    MapPin,
    Sun,
    Trash2,
    Users,
    X,
    Zap
  } from '@lucide/svelte';
  import {
    ApiClient,
    browserSession,
    demoReport,
    type AntiAdblockSettings,
    type SiteIconSettings,
    type Anomaly,
    type Attribution,
    type CollectionHealth,
    type Funnel,
    type FunnelReport,
    type Goal,
    type Journey,
    type LiveEvent,
    type Overview,
    type ReportRow,
    type ReportSubscription,
    type SearchConsoleStatus,
    type BillingStatus,
    type Site,
    type Visitor
  } from '$lib/api';
  import { applyTheme, compactNumber, duration, flagEmoji, ignoreVisitsLinks, portfolioTotals, sparklinePoints, type Theme } from '$lib/ui';
  import { appHref, parseDays, parseView, type SiteView } from '$lib/app-routes';
  import ChangeBadge from '$lib/components/rollup/ChangeBadge.svelte';
  import SiteCard from '$lib/components/rollup/SiteCard.svelte';
  import PlanCard from '$lib/components/billing/PlanCard.svelte';
  import AntiAdblockSettingsPanel from '$lib/components/AntiAdblockSettings.svelte';
  import ReportTable from '$lib/components/ReportTable.svelte';
  import TrafficChart from '$lib/components/TrafficChart.svelte';
  import InsightsView from '$lib/components/insights/InsightsView.svelte';
  import type { IconDimension } from '$lib/components/DimensionIcon.svelte';
  import SiteIcon from '$lib/components/SiteIcon.svelte';
  import SiteIconSettingsCard from '$lib/components/SiteIconSettings.svelte';
  import SpyView, { type StreamState } from '$lib/components/spy/SpyView.svelte';
  import VisitorDrawer from '$lib/components/spy/VisitorDrawer.svelte';

  type View = 'rollup' | SiteView;
  const demo = env.PUBLIC_DEMO_MODE === 'true';
  const apiBase = env.PUBLIC_API_BASE_URL || '/api';
  const api = new ApiClient(apiBase, fetch, demo);
  // An expired or revoked session must clear the stored token, or /login would bounce straight back here.
  api.onUnauthorized = () => {
    api.forgetSession();
    source?.close();
    void goto('/login');
  };
  const nav: Array<{ id: SiteView; label: string; icon: typeof Activity }> = [
    { id: 'overview', label: 'Overview', icon: Gauge },
    { id: 'insights', label: 'Insights', icon: BarChart3 },
    { id: 'spy', label: 'Spy', icon: Eye },
    { id: 'pages', label: 'Pages', icon: FileText },
    { id: 'referrers', label: 'Referrers', icon: Activity },
    { id: 'countries', label: 'Countries', icon: Globe2 },
    { id: 'regions', label: 'Regions', icon: MapPin },
    { id: 'cities', label: 'Cities', icon: MapPin },
    { id: 'devices', label: 'Devices', icon: Monitor },
    { id: 'browsers', label: 'Browsers', icon: Compass },
    { id: 'operating-systems', label: 'Operating systems', icon: Smartphone },
    { id: 'campaigns', label: 'Campaigns', icon: Zap },
    { id: 'visitors', label: 'Visitors', icon: Users },
    { id: 'goals', label: 'Goals', icon: GoalIcon },
    { id: 'settings', label: 'Settings', icon: Settings }
  ];
  let ready = $state(false);
  // Routing waits for the first site list so /app/{siteId} can resolve to a loaded site.
  let sitesLoaded = $state(false);
  let sites = $state<Site[]>([]);
  let site = $state<Site | null>(null);
  let view = $state<View>('rollup');
  let days = $state(28);
  let loading = $state(false);
  let error = $state('');
  let menuOpen = $state(false);
  let overview = $state<Overview | null>(null);
  let report = $state<ReportRow[]>([]);
  let topPages = $state<ReportRow[]>([]);
  let topReferrers = $state<ReportRow[]>([]);
  let visitors = $state<Visitor[]>([]);
  let events = $state<LiveEvent[]>([]);
  // Spy totals need every event in the last 30 minutes; `events` is a capped display feed.
  let liveWindow = $state<LiveEvent[]>([]);
  const LIVE_WINDOW_MS = 30 * 60000;
  function addToWindow(items: LiveEvent[]) {
    const since = Date.now() - LIVE_WINDOW_MS;
    const byId = new Map(liveWindow.map((item) => [item.id, item]));
    for (const item of items) byId.set(item.id, item);
    liveWindow = [...byId.values()].filter((item) => new Date(item.timestamp).getTime() >= since);
  }
  let goals = $state<Goal[]>([]);
  let journeys = $state<Journey[]>([]);
  let attribution = $state<Attribution[]>([]);
  let anomalies = $state<Anomaly[]>([]);
  let funnels = $state<Funnel[]>([]);
  let funnelReports = $state<FunnelReport[]>([]);
  type InsightSection = 'attribution' | 'journeys' | 'anomalies' | 'funnels' | 'reports';
  let insightFailures = $state(new Set<InsightSection>());
  let landingPages = $state<ReportRow[]>([]);
  let exitPages = $state<ReportRow[]>([]);
  let sources = $state<ReportRow[]>([]);
  let content = $state<ReportRow[]>([]);
  let aiReferrers = $state<ReportRow[]>([]);
  let aiCrawlers = $state<ReportRow[]>([]);
  let collectionHealth = $state<CollectionHealth | null>(null);
  let searchConsole = $state<SearchConsoleStatus | null>(null);
  let reportSubscriptions = $state<ReportSubscription[]>([]);
  let briefName = $state('Weekly marketing brief');
  let briefWebhook = $state('');
  let briefFrequency = $state<'daily' | 'weekly'>('weekly');
  let briefAnomaliesOnly = $state(false);
  let newSigningSecret = $state('');
  let paused = $state(false);
  let streamState = $state<StreamState>('connecting');
  let spyFilter = $state('');
  let selectedVisitor = $state<Visitor | null>(null);
  let source: EventSource | null = null;
  let theme = $state<Theme>('system');
  let newGoal = $state(false);
  let goalName = $state('');
  let goalTarget = $state('');
  let newSite = $state(false);
  let siteName = $state('');
  let siteDomain = $state('');
  let siteError = $state('');
  let isAdmin = $state(false);
  let initials = $state('');

  onMount(() => {
    theme = (localStorage.getItem('slimlytics_theme') ?? 'system') as Theme;
    api.useSession(browserSession);
    if (!api.accessToken && !demo) {
      void goto('/login');
      return;
    }
    if (demo) api.setToken('demo');
    ready = true;
    void loadSites();
    if (!demo)
      void api
        .me()
        .then((account) => {
          isAdmin = account.isAdmin ?? false;
          initials = account.email.slice(0, 2).toUpperCase();
        })
        .catch(() => {});
    // Keep stats fresh while the dashboard stays open (e.g. phone browsing + desktop dashboard).
    const refresh = () => {
      if (document.visibilityState !== 'visible' || loading) return;
      if (!site) {
        void refreshSitesQuietly().catch(() => {});
        void refreshBillingQuietly();
      }
      else if (view !== 'settings') void refreshViewQuietly().catch(() => {});
    };
    const interval = window.setInterval(refresh, 15_000);
    const onFocus = () => refresh();
    window.addEventListener('focus', onFocus);
    document.addEventListener('visibilitychange', onFocus);
    return () => {
      source?.close();
      window.clearInterval(interval);
      window.removeEventListener('focus', onFocus);
      document.removeEventListener('visibilitychange', onFocus);
    };
  });

  // The URL is the source of truth for the site, panel, and date range.
  // A message that must survive the redirect it triggers (e.g. an unknown site ID).
  let routeNotice = '';
  const siteParam = $derived(page.params.site);
  const viewParam = $derived(page.params.view);
  const daysParam = $derived(parseDays(page.url.searchParams.get('days')));
  $effect(() => {
    const route = { siteId: siteParam, segment: viewParam, nextDays: daysParam };
    if (!sitesLoaded) return;
    untrack(() => void applyRoute(route));
  });

  async function applyRoute({
    siteId,
    segment,
    nextDays
  }: {
    siteId: string | undefined;
    segment: string | undefined;
    nextDays: number;
  }) {
    days = nextDays;
    menuOpen = false;
    error = routeNotice;
    routeNotice = '';
    if (!siteId) {
      // Search Console's OAuth callback returns to /app?site={id}; send it to that site's settings.
      const returning = page.url.searchParams.get('site');
      if (returning) return void goto(appHref(returning, 'settings', days), { replaceState: true });
      source?.close();
      site = null;
      view = 'rollup';
      if (sitesDays !== days) await loadSites();
      return;
    }
    const next = sites.find((item) => item.id === siteId);
    const nextView = parseView(segment);
    if (!next) {
      routeNotice = 'That site was not found in your workspace.';
      return void goto(appHref(null, null, days), { replaceState: true });
    }
    if (!nextView) return void goto(appHref(siteId, 'overview', days), { replaceState: true });
    site = next;
    view = nextView;
    await loadView();
  }
  function changeDays(next: number) {
    void goto(appHref(site?.id, site ? (view as SiteView) : null, next), { keepFocus: true });
  }

  async function logout() {
    source?.close();
    await api.logout();
    void goto('/');
  }
  // Hosted-plan billing; stays { enabled: false } on self-hosted installs.
  let billingStatus = $state<BillingStatus>({ enabled: false });
  // True once the server has answered; until then a failed load is retried on refresh.
  let billingKnown = false;
  let billingBusy = $state(false);
  let billingNotice = $state('');
  async function loadBilling() {
    try {
      billingStatus = await api.billing();
      billingKnown = true;
    } catch {
      // Keep the last good status on a transient failure; only an unknown state hides billing.
      if (!billingKnown) billingStatus = { enabled: false };
    }
  }
  // Keeps usage meters and limit warnings current (and rolls over at UTC midnight) without
  // hiding the card on a transient error.
  async function refreshBillingQuietly() {
    if (billingKnown && !billingStatus.enabled) return;
    try {
      billingStatus = await api.billing();
      billingKnown = true;
    } catch {
      /* keep the last known status */
    }
  }
  async function startCheckout(plan: string, interval: 'month' | 'year') {
    billingBusy = true;
    try {
      const { url } = await api.billingCheckout(plan, interval);
      location.assign(url);
    } catch (reason) {
      error = reason instanceof Error ? reason.message : 'Could not start checkout.';
      billingBusy = false;
    }
  }
  async function openBillingPortal() {
    billingBusy = true;
    try {
      location.assign((await api.billingPortal()).url);
    } catch (reason) {
      error = reason instanceof Error ? reason.message : 'Could not open billing.';
      billingBusy = false;
    }
  }
  // Returning from Stripe Checkout: the webhook updates the plan asynchronously, so re-check
  // briefly until it lands.
  async function confirmCheckout() {
    billingNotice = 'Thanks! Your subscription is being activated…';
    // Activated means a subscription in good standing, whether the webhook landed before the
    // page's first billing fetch or during the polling below.
    const isActive = () =>
      billingStatus.planSource === 'stripe' &&
      ['active', 'trialing'].includes(billingStatus.subscriptionStatus ?? '');
    let activated = isActive();
    for (let attempt = 0; attempt < 8 && !activated; attempt++) {
      await new Promise((resolve) => setTimeout(resolve, 1500));
      await loadBilling();
      activated = isActive();
    }
    if (!activated) {
      // Keep ?billing=success so a reload checks again; the regular refresh also picks it up.
      billingNotice = 'Payment received. Your plan is still activating — this can take a minute; refresh to check.';
      return;
    }
    billingNotice = `You’re on the ${billingStatus.plan?.name} plan.`;
    // Drop ?billing=success only if the user is still on the page they returned to.
    if (page.url.searchParams.get('billing') === 'success' && !site) {
      replaceState(appHref(null, null, days), page.state);
    }
  }

  async function loadSites() {
    loading = true;
    error = '';
    void loadBilling().then(() => {
      if (page.url.searchParams.get('billing') === 'success') void confirmCheckout();
    });
    try {
      await refreshSitesQuietly();
    } catch (reason) {
      error = reason instanceof Error ? reason.message : 'Could not load sites.';
    } finally {
      loading = false;
      sitesLoaded = true;
    }
  }
  // The date range the workspace overviews were loaded with. A site view can change the
  // range, so returning to All sites must reload when it no longer matches.
  let sitesDays: number | null = null;
  async function refreshSitesQuietly() {
    const range = days;
    const loaded = await api.sites();
    sites = await Promise.all(
      loaded.map(async (item) => {
        try {
          return { ...item, overview: await api.overview(item.id, range) };
        } catch {
          return item;
        }
      })
    );
    sitesDays = range;
  }
  async function loadView() {
    if (!site) return;
    loading = true;
    error = '';
    source?.close();
    try {
      await refreshViewQuietly(true);
    } catch (reason) {
      error = reason instanceof Error ? reason.message : 'Could not load analytics.';
    } finally {
      loading = false;
    }
  }
  async function refreshViewQuietly(resetStream = false) {
    if (!site) return;
    if (view === 'overview') {
      const [nextOverview, pages, referrers] = await Promise.all([
        api.overview(site.id, days),
        api.report(site.id, 'pages', days),
        api.report(site.id, 'referrers', days)
      ]);
      overview = nextOverview;
      topPages = (demo ? demoReport('pages') : pages).slice(0, 5);
      topReferrers = (demo ? demoReport('referrers') : referrers).slice(0, 5);
      // Keep sidebar "online now" in sync.
      sites = sites.map((item) =>
        item.id === site?.id ? { ...item, overview: nextOverview } : item
      );
      // That entry now holds this range; the workspace cache no longer has one consistent range.
      if (days !== sitesDays) sitesDays = null;
    } else if (view === 'insights') {
      // Load sections independently so one failing query cannot blank the whole page.
      const id = site.id;
      const [journeysResult, attributionResult, anomaliesResult, funnelsResult, reportsResult] =
        await Promise.allSettled([
          api.journeys(id, days),
          api.attribution(id, days),
          api.anomalies(id, days),
          api.funnels(id).then(async (list) => ({
            list,
            reports: await Promise.all(list.map((funnel) => api.funnelReport(id, funnel.id, days)))
          })),
          Promise.all(
            ['landing-pages', 'exit-pages', 'sources', 'content', 'ai-referrers', 'ai-crawlers'].map(
              (type) => api.report(id, type, days)
            )
          )
        ]);
      const failed = new Set<InsightSection>();
      if (journeysResult.status === 'fulfilled') journeys = journeysResult.value;
      else {
        journeys = [];
        failed.add('journeys');
      }
      if (attributionResult.status === 'fulfilled') attribution = attributionResult.value;
      else {
        attribution = [];
        failed.add('attribution');
      }
      if (anomaliesResult.status === 'fulfilled') anomalies = anomaliesResult.value;
      else {
        anomalies = [];
        failed.add('anomalies');
      }
      if (funnelsResult.status === 'fulfilled') {
        funnels = funnelsResult.value.list;
        funnelReports = funnelsResult.value.reports;
      } else {
        funnels = [];
        funnelReports = [];
        failed.add('funnels');
      }
      if (reportsResult.status === 'fulfilled')
        [landingPages, exitPages, sources, content, aiReferrers, aiCrawlers] = reportsResult.value;
      else {
        [landingPages, exitPages, sources, content, aiReferrers, aiCrawlers] = [[], [], [], [], [], []];
        failed.add('reports');
      }
      insightFailures = failed;
    } else if (
      [
        'pages',
        'referrers',
        'countries',
        'regions',
        'cities',
        'devices',
        'browsers',
        'operating-systems',
        'campaigns'
      ].includes(view)
    )
      report = await api.report(site.id, view, days);
    else if (view === 'visitors') visitors = await api.visitors(site.id);
    else if (view === 'spy') {
      events = await api.events(site.id);
      if (resetStream) liveWindow = [];
      addToWindow(events);
      visitors = await api.visitors(site.id);
      if (resetStream || !source || source.readyState === EventSource.CLOSED) connectSpy();
    } else if (view === 'goals') goals = await api.goals(site.id);
    else if (view === 'settings')
      [collectionHealth, searchConsole, reportSubscriptions] = await Promise.all([
        api.collectionHealth(site.id),
        api.searchConsoleStatus(site.id),
        api.reportSubscriptions(site.id)
      ]);
  }
  function connectSpy() {
    if (!site || paused || demo || typeof EventSource === 'undefined') {
      streamState = paused ? 'paused' : 'offline';
      return;
    }
    source?.close();
    streamState = 'connecting';
    const streamToken = api.accessToken;
    source = new EventSource(api.streamUrl(site.id, streamToken));
    const receive = ({ data }: MessageEvent<string>) => {
      try {
        const item = JSON.parse(data) as LiveEvent;
        // The stream replays recent events on connect; skip ones already loaded.
        if (events.some((existing) => existing.id === item.id)) return;
        events = [item, ...events].slice(0, 100);
        addToWindow([item]);
      } catch {
        /* malformed event */
      }
    };
    source.onopen = () => (streamState = 'live');
    source.onmessage = receive;
    source.addEventListener('event', receive as EventListener);
    // Browsers reconnect EventSource automatically; only tear down when we mean to.
    source.onerror = () => {
      if (paused || view !== 'spy') return void source?.close();
      streamState = 'reconnecting';
      // An expired access token closes the stream for good: renew it, then reconnect.
      if (source?.readyState === EventSource.CLOSED)
        void api
          .refreshSession(streamToken)
          .then((renewed) => {
            if (!renewed) api.onUnauthorized?.();
            else if (view === 'spy' && !paused) connectSpy();
          })
          // A network or server hiccup: try the stream again shortly.
          .catch(() =>
            window.setTimeout(() => {
              if (view === 'spy' && !paused && source?.readyState === EventSource.CLOSED) connectSpy();
            }, 5000)
          );
    };
  }
  // Visitors who arrived after the page loaded aren't in `visitors` yet; describe them
  // from their latest streamed event instead.
  function selectStreamVisitor(id: string) {
    const known = visitors.find((visitor) => visitor.id === id);
    const latest = events.find((item) => item.visitorId === id);
    selectedVisitor =
      known ??
      (latest
        ? { id, country: latest.country ?? '', city: latest.city, page: latest.page, lastSeen: latest.timestamp }
        : null);
  }
  function toggleSpy() {
    paused = !paused;
    if (paused) {
      source?.close();
      streamState = 'paused';
    } else connectSpy();
  }
  function updateTheme(next: Theme) {
    theme = next;
    localStorage.setItem('slimlytics_theme', next);
    applyTheme(next);
  }
  async function saveAntiAdblock(settings: AntiAdblockSettings) {
    if (!site) return;
    const updated = await api.updateAntiAdblock(site.id, settings);
    const next = { ...updated, overview: site.overview };
    site = next;
    sites = sites.map((item) => (item.id === next.id ? { ...next, overview: item.overview } : item));
  }
  async function saveSiteIcon(settings: SiteIconSettings) {
    if (!site) return;
    const updated = await api.updateSiteIcon(site.id, settings);
    const next = { ...updated, overview: site.overview };
    site = next;
    sites = sites.map((item) => (item.id === next.id ? { ...next, overview: item.overview } : item));
  }
  async function connectSearchConsole() {
    if (!site) return;
    const { authorizationUrl } = await api.connectSearchConsole(site.id);
    location.assign(authorizationUrl);
  }
  async function syncSearchConsole() {
    if (!site) return;
    loading = true;
    try {
      await api.syncSearchConsole(site.id, days);
      searchConsole = await api.searchConsoleStatus(site.id);
    } finally {
      loading = false;
    }
  }
  async function disconnectSearchConsole() {
    if (!site || !confirm('Disconnect Search Console and remove its cached metrics?')) return;
    await api.disconnectSearchConsole(site.id);
    searchConsole = await api.searchConsoleStatus(site.id);
  }
  async function createBrief(event: SubmitEvent) {
    event.preventDefault();
    if (!site || !briefName.trim() || !briefWebhook.trim()) return;
    const created = await api.createReportSubscription(site.id, {
      name: briefName.trim(), webhookUrl: briefWebhook.trim(), frequency: briefFrequency,
      anomalyOnly: briefAnomaliesOnly, enabled: true
    });
    newSigningSecret = created.signingSecret ?? '';
    reportSubscriptions = [...reportSubscriptions, created];
    briefWebhook = '';
  }
  async function toggleBrief(subscription: ReportSubscription) {
    if (!site) return;
    const updated = await api.updateReportSubscription(site.id, {
      ...subscription, enabled: !subscription.enabled
    });
    reportSubscriptions = reportSubscriptions.map((item) => item.id === updated.id ? updated : item);
  }
  async function deliverBrief(subscription: ReportSubscription) {
    if (!site) return;
    await api.deliverReportSubscription(site.id, subscription.id);
    reportSubscriptions = await api.reportSubscriptions(site.id);
  }
  async function deleteBrief(subscription: ReportSubscription) {
    if (!site || !confirm(`Delete ${subscription.name}?`)) return;
    await api.deleteReportSubscription(site.id, subscription.id);
    reportSubscriptions = reportSubscriptions.filter((item) => item.id !== subscription.id);
  }
  async function rotateServerKey() {
    if (!site || !confirm('Rotate the server ingestion key? Existing log shippers will stop working.')) return;
    const result = await api.rotateServerKey(site.id);
    site = { ...site, serverWriteKey: result.serverWriteKey };
    sites = sites.map((item) => item.id === site?.id ? { ...item, serverWriteKey: result.serverWriteKey } : item);
  }
  async function addGoal() {
    if (!site || !goalName || !goalTarget) return;
    const goal = await api.createGoal(site.id, { name: goalName, target: goalTarget, type: 'event' });
    goals = [...goals, goal];
    newGoal = false;
    goalName = '';
    goalTarget = '';
  }
  async function addSite() {
    if (!siteName.trim() || !siteDomain.trim()) return;
    siteError = '';
    try {
      const raw = siteDomain.trim().replace(/\/$/, '');
      const origin = /^https?:\/\//i.test(raw) ? new URL(raw).origin : `https://${raw}`;
      const domain = new URL(origin).host;
      const created = await api.createSite({
        name: siteName.trim(),
        domain,
        allowedOrigins: [origin]
      });
      sites = [...sites, { ...created, overview: await api.overview(created.id, days) }];
      void refreshBillingQuietly();
      newSite = false;
      siteName = '';
      siteDomain = '';
      await goto(appHref(created.id, 'overview', days));
    } catch (reason) {
      siteError = reason instanceof Error ? reason.message : 'Could not create site.';
    }
  }
  async function downloadCsv() {
    if (!site) return;
    try {
      const blob = await api.downloadExport(site.id, days);
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = `${site.domain}-events.csv`;
      anchor.click();
      URL.revokeObjectURL(url);
    } catch (reason) {
      error = reason instanceof Error ? reason.message : 'Could not export events.';
    }
  }
  let siteSort = $state<'visitors' | 'change' | 'name'>('visitors');
  // Where the measured site lives, for "open page in a new tab" links on path reports.
  const iconDimensions: IconDimension[] = ['countries', 'devices', 'browsers', 'operating-systems'];
  const siteOrigin = $derived(site ? (site.allowedOrigins?.[0] ?? `https://${site.domain}`) : undefined);
  const totals = $derived(portfolioTotals(sites.map((item) => item.overview)));
  const activeSites = $derived(sites.filter((item) => (item.overview?.visitors ?? 0) > 0).length);
  const unavailableSites = $derived(sites.filter((item) => !item.overview).length);
  const sortedSites = $derived(
    [...sites].sort((a, b) => {
      if (siteSort === 'name') return a.name.localeCompare(b.name);
      if (siteSort === 'change')
        return Math.abs(b.overview?.change ?? 0) - Math.abs(a.overview?.change ?? 0);
      return (b.overview?.visitors ?? 0) - (a.overview?.visitors ?? 0) || a.name.localeCompare(b.name);
    })
  );
</script>

<svelte:head>
  <title
    >{site
      ? `${nav.find((item) => item.id === view)?.label ?? 'Overview'} · ${site.domain}`
      : 'All sites'} · Slimlytics</title
  >
</svelte:head>

{#if ready}
  <div class="app-shell">
    <aside class:open={menuOpen} aria-label="Primary navigation">
      <div class="sidebar-brand">
        <a class="brand" href={appHref(null, null, days)}>
          <img class="mascot-logo" src="/images/mascot/meerkat-logo.webp" width="38" height="38" alt="" /><strong>Slimlytics</strong>
        </a>
        <button class="icon-button close-menu" aria-label="Close menu" onclick={() => (menuOpen = false)}
          ><X /></button
        >
      </div>
      <a class="site-picker" href={appHref(null, null, days)} aria-label="All sites">
        {#if site}<SiteIcon {site} {apiBase} />{:else}<span class="site-avatar">ALL</span>{/if}
        <span
          ><small>{site ? 'Current site' : 'Workspace'}</small><strong
            >{site?.name ?? 'All sites'}</strong
          ></span
        ><ChevronDown size={15} aria-hidden="true" />
      </a>
      {#if site}
        <nav>
          {#each nav as item}
            <a
              class:active={view === item.id}
              href={appHref(site.id, item.id, days)}
              aria-current={view === item.id ? 'page' : undefined}
              ><item.icon size={17} aria-hidden="true" /><span>{item.label}</span>{#if item.id === 'spy'}<i
                ></i
              >{/if}</a
            >
          {/each}
        </nav>
      {:else}
        <nav>
          <a class="active" href={appHref(null, null, days)} aria-current="page"
            ><LayoutDashboard size={17} aria-hidden="true" />All sites</a
          >
        </nav>
      {/if}
      <div class="sidebar-foot">
        <div class="online">
          <span></span
          >{sites.reduce((sum, current) => sum + (current.overview?.currentOnline ?? 0), 0)} online
          now
        </div>
        <a class="sidebar-link" href="/account"><KeyRound size={16} aria-hidden="true" />Account security</a>
        {#if isAdmin}
          <a class="sidebar-link" href="/admin"><ShieldCheck size={16} aria-hidden="true" />Admin</a>
        {/if}
        <button onclick={() => void logout()}><LogOut size={16} />Sign out</button>
        <a class="geo-credit" href="https://db-ip.com" target="_blank" rel="noopener noreferrer"
          >IP geolocation by DB-IP</a
        >
      </div>
    </aside>
    {#if menuOpen}
      <button class="scrim" aria-label="Close menu" onclick={() => (menuOpen = false)}></button>
    {/if}
    <main id="main" class="workspace">
      <header class="topbar">
        <button class="icon-button menu-button" aria-label="Open menu" onclick={() => (menuOpen = true)}
          ><Menu /></button
        >
        <div>
          <p class="breadcrumb">{site ? site.domain : 'Workspace'} <span>/</span></p>
          <h1>{site ? (nav.find((item) => item.id === view)?.label ?? 'Overview') : 'All sites'}</h1>
        </div>
        <div class="top-actions">
          <label class="date-picker"
            ><CalendarDays size={16} /><span class="sr-only">Date range</span
            ><select value={days} onchange={(event) => changeDays(Number(event.currentTarget.value))}
              ><option value={7}>Last 7 days</option><option value={28}>Last 28 days</option
              ><option value={90}>Last 90 days</option></select
            ></label
          ><button class="icon-button" aria-label="Notifications"><Bell size={18} /></button
          ><a class="avatar" href="/account" aria-label="Account security">{initials || 'ME'}</a>
        </div>
      </header>
      {#if error}
        <div class="alert page-alert" role="alert">
          <span>{error}</span><button onclick={() => void (site ? loadView() : loadSites())}>Retry</button>
        </div>
      {/if}
      {#if loading}
        <div class="loading" role="status"><span></span><p>Loading analytics…</p></div>
      {:else if !site}
        <section class="page-head">
          <div>
            <p class="eyebrow">Portfolio pulse</p>
            <h2>Your sites at a glance</h2>
            <p class="muted">Traffic across all properties during the last {days} days.</p>
          </div>
          <button class="primary" onclick={() => (newSite = true)}><Plus size={16} /> Add site</button>
        </section>
        <!-- Account-level billing shows even with no sites, so subscribers can always manage it. -->
        {#if billingNotice}<p class="success-message billing-notice" role="status">{billingNotice}</p>{/if}
        {#if billingStatus.enabled && billingStatus.plan}
          <PlanCard status={billingStatus} busy={billingBusy} onCheckout={startCheckout} onPortal={openBillingPortal} />
        {/if}
        {#if sites.length}
          <section class="portfolio-summary" aria-label="Workspace totals">
            <div>
              <span class="label"><Users size={15} aria-hidden="true" /> Visitors</span>
              <strong>{compactNumber(totals.visitors)}</strong>
              <ChangeBadge change={totals.change} />
            </div>
            <div>
              <span class="label"><FileText size={15} aria-hidden="true" /> Page views</span>
              <strong>{compactNumber(totals.pageViews)}</strong>
              <small>{totals.visitors ? (totals.pageViews / totals.visitors).toFixed(1) : '0'} per visitor</small>
            </div>
            <div>
              <span class="label"><Radio size={15} aria-hidden="true" /> Online now</span>
              <strong>{totals.online}</strong>
              <small>across {sites.length} {sites.length === 1 ? 'site' : 'sites'}</small>
            </div>
            <div>
              <span class="label"><Layers size={15} aria-hidden="true" /> Active sites</span>
              <strong>{activeSites}<span class="of">/{sites.length}</span></strong>
              <small>with visits in {days} days</small>
            </div>
          </section>
          {#if unavailableSites}
            <p class="rollup-note" role="status">
              Totals exclude {unavailableSites}
              {unavailableSites === 1 ? 'site whose stats' : 'sites whose stats'} couldn’t load.
            </p>
          {/if}
          <div class="rollup-toolbar">
            <h3>Sites</h3>
            <label class="sort-picker"
              ><span>Sort by</span><select bind:value={siteSort}
                ><option value="visitors">Most visitors</option><option value="change"
                  >Biggest change</option
                ><option value="name">Name</option></select
              ></label
            >
          </div>
          <div class="site-grid rollup-grid">
            {#each sortedSites as item (item.id)}
              <SiteCard site={item} {days} {apiBase} />
            {/each}
            <button class="add-site-card" onclick={() => (newSite = true)}>
              <span><Plus size={20} aria-hidden="true" /></span>
              <strong>Add a site</strong>
              <small>Start measuring another property</small>
            </button>
          </div>
        {:else}
          <div class="empty large">
            <Globe2 size={34} /><h2>No sites yet</h2>
            <p>Add your first website to begin measuring private, useful analytics.</p>
            <button class="primary" onclick={() => (newSite = true)}
              ><Plus />Add your first site</button
            >
          </div>
        {/if}
        {#if newSite}
          <div class="modal-backdrop" role="presentation"
            ><form
              class="modal"
              onsubmit={(event) => {
                event.preventDefault();
                void addSite();
              }}
              ><div class="panel-head">
                <h2>Add a site</h2>
                <button
                  type="button"
                  class="icon-button"
                  onclick={() => (newSite = false)}
                  aria-label="Close add site dialog"><X /></button
                >
              </div>
              <label>Site name<input bind:value={siteName} required placeholder="My website" /></label
              ><label
                >Domain<input bind:value={siteDomain} required placeholder="example.com" /></label
              >{#if siteError}<div class="alert" role="alert">{siteError}</div>{/if}<p class="muted">
                HTTPS will be used as the initial allowed tracking origin.
              </p>
              <div class="modal-actions">
                <button type="button" class="secondary" onclick={() => (newSite = false)}>Cancel</button
                ><button class="primary">Create site</button>
              </div></form
            ></div
          >
        {/if}
      {:else if view === 'overview' && overview}
        <section class="metric-grid overview-metrics" aria-label="Key metrics">
          <article class="metric-card featured">
            <div><small>Visitors</small><Users size={17} aria-hidden="true" /></div>
            <strong>{overview.visitors.toLocaleString()}</strong>
            <ChangeBadge change={overview.change} />
          </article>
          <article class="metric-card">
            <div><small>Sessions</small><Activity size={17} aria-hidden="true" /></div>
            <strong>{overview.sessions.toLocaleString()}</strong>
            <span class="metric-note"
              >{overview.visitors ? (overview.sessions / overview.visitors).toFixed(1) : '0'} per visitor</span
            >
          </article>
          <article class="metric-card">
            <div><small>Page views</small><FileText size={17} aria-hidden="true" /></div>
            <strong>{overview.pageViews.toLocaleString()}</strong>
            <span class="metric-note"
              >{overview.sessions ? (overview.pageViews / overview.sessions).toFixed(1) : '0'} per session</span
            >
          </article>
          <article class="metric-card">
            <div><small>Bounce rate</small><Gauge size={17} aria-hidden="true" /></div>
            <strong>{Math.round(overview.bounceRate)}%</strong>
            <span class="metric-meter" aria-hidden="true"><i style={`width:${Math.min(100, overview.bounceRate)}%`}></i></span>
            <span class="metric-note">single-page sessions</span>
          </article>
          <article class="metric-card">
            <div><small>Avg. visit</small><Timer size={17} aria-hidden="true" /></div>
            <strong>{duration(overview.avgDuration)}</strong>
            <span class="metric-note">per session</span>
          </article>
          <article class="metric-card" class:live-now={overview.currentOnline > 0}>
            <div><small>Online now</small><Radio size={17} aria-hidden="true" /></div>
            <strong>{overview.currentOnline}</strong>
            <a class="metric-note metric-link" href={appHref(site.id, 'spy', days)}
              >{overview.currentOnline ? 'Watch live' : 'Open Spy'} <ArrowRight size={12} aria-hidden="true" /></a
            >
          </article>
        </section>
        <section class="panel chart-panel">
          <div class="panel-head">
            <div>
              <p class="eyebrow">Traffic volume</p>
              <h2>Visitors & page views</h2>
            </div>
            <div class="legend" aria-hidden="true">
              <span><i class="visitors"></i>Visitors</span><span><i class="views"></i>Page views</span>
            </div>
          </div>
          <TrafficChart trend={overview.trend} />
        </section>
        <div class="two-col">
          <ReportTable
            title="Top pages"
            rows={topPages}
            moreHref={appHref(site.id, 'pages', days)}
            pageOrigin={siteOrigin}
          />
          <ReportTable
            title="Top referrers"
            rows={topReferrers}
            moreHref={appHref(site.id, 'referrers', days)}
            linkHosts
          />
        </div>
      {:else if view === 'spy'}
        <SpyView
          {events}
          windowEvents={liveWindow}
          {visitors}
          {streamState}
          bind:filter={spyFilter}
          pageOrigin={siteOrigin}
          onToggle={toggleSpy}
          onSelect={selectStreamVisitor}
        />
      {:else if view === 'insights'}
        <InsightsView
          {attribution}
          {journeys}
          {anomalies}
          {funnelReports}
          {landingPages}
          {exitPages}
          {sources}
          {content}
          {aiReferrers}
          {aiCrawlers}
          pageOrigin={siteOrigin}
          failed={insightFailures}
          retry={() => void loadView()}
        />
      {:else if
        [
          'pages',
          'referrers',
          'countries',
          'regions',
          'cities',
          'devices',
          'browsers',
          'operating-systems',
          'campaigns'
        ].includes(view)}
        <section class="page-head">
          <div>
            <p class="eyebrow">Acquisition detail</p>
            <h2>{nav.find((item) => item.id === view)?.label}</h2>
            <p class="muted">Ranked by page views during the selected period.</p>
          </div>
          <button class="secondary button" onclick={() => void downloadCsv()}
            ><Download size={15} />Export CSV</button
          >
        </section>
        <ReportTable
          title={`${nav.find((item) => item.id === view)?.label} report`}
          rows={report}
          pageOrigin={view === 'pages' ? siteOrigin : undefined}
          linkHosts={view === 'referrers'}
          iconDimension={iconDimensions.find((dimension) => dimension === view)}
        />
      {:else if view === 'visitors'}
        <section class="panel">
          <div class="panel-head">
            <div>
              <p class="eyebrow">Audience</p>
              <h2>Recent visitors</h2>
            </div>
            <span>{visitors.length} visitors</span>
          </div>
          {#if visitors.length}
            <div class="visitor-list">
              {#each visitors as visitor}
                <button onclick={() => (selectedVisitor = visitor)}
                  ><span class="country-code" title={visitor.country.slice(0, 2).toUpperCase()}
                    ><span aria-hidden="true">{flagEmoji(visitor.country)}</span></span
                  ><span
                    ><strong>{visitor.city ?? 'Unknown'}, {visitor.country}</strong><small
                      >{visitor.device} · {visitor.browser} · {visitor.page}</small
                    ></span
                  ><span>{visitor.sessions ?? 1} sessions</span></button
                >
              {/each}
            </div>
          {:else}
            <div class="empty"><Users /><p>No visitors in this period.</p></div>
          {/if}
        </section>
      {:else if view === 'goals'}
        <section class="page-head">
          <div>
            <p class="eyebrow">Outcomes</p>
            <h2>Goals</h2>
            <p class="muted">Measure the actions that matter, not just clicks.</p>
          </div>
          <button class="primary" onclick={() => (newGoal = true)}
            ><Plus size={16} />New goal</button
          >
        </section>
        <div class="goal-grid">
          {#each goals as goal}
            <article class="panel goal-card">
              <span><GoalIcon /></span>
              <div>
                <small>{goal.type}</small>
                <h3>{goal.name}</h3>
                <code>{goal.target}</code>
              </div>
              <div>
                <strong>{goal.conversions ?? 0}</strong><small>conversions</small>
              </div>
              <div>
                <strong>{goal.conversionRate ?? 0}%</strong><small>conversion rate</small>
              </div>
            </article>
          {/each}
        </div>
        {#if newGoal}
          <div class="modal-backdrop" role="presentation"
            ><form
              class="modal"
              onsubmit={(event) => {
                event.preventDefault();
                void addGoal();
              }}
              ><div class="panel-head">
                <h2>Create goal</h2>
                <button type="button" class="icon-button" onclick={() => (newGoal = false)}
                  ><X /></button
                >
              </div>
              <label
                >Goal name<input
                  bind:value={goalName}
                  required
                  placeholder="Newsletter signup"
                /></label
              ><label
                >Event name<input bind:value={goalTarget} required placeholder="signup" /></label
              >
              <div class="modal-actions">
                <button type="button" class="secondary" onclick={() => (newGoal = false)}>Cancel</button
                ><button class="primary">Create goal</button>
              </div></form
            ></div
          >
        {/if}
      {:else if view === 'settings'}
        <section class="settings-grid">
          <div class="panel settings-card">
            <p class="eyebrow">Appearance</p>
            <h2>Theme</h2>
            <p class="muted">Use your system preference or override it.</p>
            <div class="theme-options">
              {#each [{ id: 'light', label: 'Light', icon: Sun }, { id: 'dark', label: 'Dark', icon: Moon }, { id: 'system', label: 'System', icon: Smartphone }] as option}
                <button
                  class:active={theme === option.id}
                  onclick={() => updateTheme(option.id as Theme)}
                  ><option.icon /><span>{option.label}</span></button
                >
              {/each}
            </div>
          </div>
          <div class="panel settings-card">
            <p class="eyebrow">Site profile</p>
            <h2>{site.name}</h2>
            <dl>
              <div>
                <dt>Domain</dt>
                <dd>{site.domain}</dd>
              </div>
              <div>
                <dt>Write key</dt>
                <dd><code>{site.writeKey}</code></dd>
              </div>
              <div>
                <dt>Timezone</dt>
                <dd>{site.timezone ?? 'UTC'}</dd>
              </div>
            </dl>
          </div>
          <SiteIconSettingsCard {site} {apiBase} save={saveSiteIcon} />
          <div class="panel settings-card">
            <p class="eyebrow">Collection</p>
            <h2>{collectionHealth?.lastAcceptedAt ? 'Receiving events' : 'Waiting for events'}</h2>
            <dl>
              <div>
                <dt>Accepted</dt>
                <dd>{collectionHealth?.acceptedTotal.toLocaleString() ?? 0}</dd>
              </div>
              <div>
                <dt>Rejected</dt>
                <dd>{collectionHealth?.rejectedTotal.toLocaleString() ?? 0}</dd>
              </div>
              <div>
                <dt>Last event</dt>
                <dd>{collectionHealth?.lastAcceptedAt
                    ? new Date(collectionHealth.lastAcceptedAt).toLocaleString()
                    : 'Never'}</dd>
              </div>
              <div>
                <dt>Tracker</dt>
                <dd>{collectionHealth?.lastTrackerVersion ?? 'Unknown'}</dd>
              </div>
              {#if collectionHealth?.lastRejectionCode}
                <div>
                  <dt>Last rejection</dt>
                  <dd><code>{collectionHealth.lastRejectionCode}</code></dd>
                </div>
              {/if}
            </dl>
          </div>
          {#if siteOrigin && ignoreVisitsLinks(siteOrigin)}
            {@const ignoreLinks = ignoreVisitsLinks(siteOrigin)!}
            <div class="panel settings-card">
              <p class="eyebrow">Your own traffic</p>
              <h2>Ignore my visits</h2>
              <p class="muted">
                Open this link once in each browser you use to keep your own visits out of reports. It saves
                a flag in that browser only. No cookies, and nothing is sent to Slimlytics.
              </p>
              <div class="test-links">
                <a class="button primary" href={ignoreLinks.ignore} target="_blank" rel="noopener">Ignore this browser</a>
                <a class="button secondary" href={ignoreLinks.resume} target="_blank" rel="noopener">Count me again</a>
              </div>
            </div>
          {/if}
          <div class="panel settings-card">
            <p class="eyebrow">Server collection</p>
            <h2>Request ingestion</h2>
            <dl>
              <div>
                <dt>Endpoint</dt>
                <dd><code>/api/ingest</code></dd>
              </div>
              <div>
                <dt>Server key</dt>
                <dd><code>{site.serverWriteKey}</code></dd>
              </div>
              <div>
                <dt>Batch limit</dt>
                <dd>100 requests</dd>
              </div>
            </dl>
            <div class="test-links">
              <button class="secondary" onclick={() => void rotateServerKey()}>Rotate key</button>
            </div>
          </div>
          <div class="panel settings-card brief-settings">
            <p class="eyebrow">Delivery</p>
            <h2>Marketing briefs</h2>
            <form class="brief-form" onsubmit={createBrief}>
              <label>Name<input bind:value={briefName} maxlength="120" required /></label>
              <label>Webhook URL<input bind:value={briefWebhook} type="url" inputmode="url" placeholder="https://hooks.example.com/report" required /></label>
              <label>Frequency<select bind:value={briefFrequency}><option value="daily">Daily</option><option value="weekly">Weekly</option></select></label>
              <label class="check-field"><input type="checkbox" bind:checked={briefAnomaliesOnly} />Anomalies only</label>
              <button class="primary"><Plus />Create</button>
            </form>
            {#if newSigningSecret}
              <p class="success-message">Signing secret: <code>{newSigningSecret}</code></p>
            {/if}
            {#if reportSubscriptions.length}
              <ul class="brief-list">
                {#each reportSubscriptions as subscription}
                  <li>
                    <span><strong>{subscription.name}</strong><small>{subscription.frequency} · {subscription.lastStatus ?? 'pending'}</small></span>
                    <button class="icon-button" title="Send now" aria-label="Send now" onclick={() => void deliverBrief(subscription)}><Send /></button>
                    <button class="secondary compact" onclick={() => void toggleBrief(subscription)}>{subscription.enabled ? 'Pause' : 'Enable'}</button>
                    <button class="icon-button" title="Delete" aria-label="Delete" onclick={() => void deleteBrief(subscription)}><Trash2 /></button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
          <div class="panel settings-card">
            <p class="eyebrow">Organic search</p>
            <h2>Google Search Console</h2>
            {#if !searchConsole?.configured}
              <p class="muted">OAuth credentials are not configured on this deployment.</p>
            {:else if !searchConsole.connected}
              <p class="muted">Connect verified search properties for query and page performance.</p>
              <div class="test-links">
                <button class="primary" onclick={() => void connectSearchConsole()}>Connect</button>
              </div>
            {:else}
              <dl>
                <div><dt>Property</dt><dd>{searchConsole.propertyUrl ?? 'No match'}</dd></div>
                <div>
                  <dt>Last sync</dt>
                  <dd>{searchConsole.lastSyncedAt
                      ? new Date(searchConsole.lastSyncedAt).toLocaleString()
                      : 'Never'}</dd>
                </div>
                {#if searchConsole.lastError}
                  <div><dt>Status</dt><dd>{searchConsole.lastError}</dd></div>
                {/if}
              </dl>
              <div class="test-links">
                <button class="primary" onclick={() => void syncSearchConsole()}>Sync now</button>
                <button class="secondary" onclick={() => void disconnectSearchConsole()}
                  >Disconnect</button
                >
              </div>
            {/if}
          </div>
          <div class="panel settings-card installation-card">
            {#key site.id}
              <AntiAdblockSettingsPanel
                {site}
                analyticsOrigin={typeof location === 'undefined'
                  ? 'https://slimlytics.com'
                  : location.origin}
                save={saveAntiAdblock}
              />
            {/key}
          </div>
        </section>
      {/if}
      {#if selectedVisitor && site}
        <VisitorDrawer visitor={selectedVisitor} {events} onClose={() => (selectedVisitor = null)} />
      {/if}
    </main>
  </div>
{/if}
