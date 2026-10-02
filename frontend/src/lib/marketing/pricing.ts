export interface PricingPlan {
  id: string;
  name: string;
  tagline: string;
  price: string;
  priceNote: string;
  /** Annual billing, shown under the monthly price. */
  annualNote?: string;
  highlighted?: boolean;
  ctaLabel: string;
  ctaHref: string;
  features: string[];
}

/**
 * Hosted tiers. The enforced limits and prices live in the backend plan config
 * (config/plans.example.json, docs/BILLING.md); keep the two in sync. Limits are daily page views (like Clicky's) so the plans compare directly, and every
 * feature is included on every plan: tiers differ only in sites, volume, and support.
 */
export const pricingPlans: PricingPlan[] = [
  {
    id: 'self-hosted',
    name: 'Self-hosted',
    tagline: 'The full open-source product on your own servers.',
    price: '$0',
    priceNote: 'free forever',
    ctaLabel: 'Read the setup guide',
    ctaHref: '/docs',
    features: [
      'Unlimited sites and page views',
      'Every feature included',
      'Docker Compose setup in minutes',
      'Your servers, your data',
      'Community support'
    ]
  },
  {
    id: 'free',
    name: 'Free',
    tagline: 'Hosted analytics for a personal site.',
    price: '$0',
    priceNote: 'per month',
    ctaLabel: 'Get started free',
    ctaHref: '/register?plan=free',
    features: [
      '1 website',
      '3,000 page views / day',
      'Every feature included',
      'MCP server, CLI, and API',
      'Community support'
    ]
  },
  {
    id: 'pro',
    name: 'Pro',
    tagline: 'For independent makers with a handful of sites.',
    price: '$7',
    priceNote: 'per month',
    annualNote: 'or $56 / year — save 33%',
    highlighted: true,
    ctaLabel: 'Start Pro',
    ctaHref: '/register?plan=pro',
    features: [
      '10 websites',
      '30,000 page views / day',
      'Every feature included',
      'MCP server, CLI, and API',
      'Email support'
    ]
  },
  {
    id: 'business',
    name: 'Business',
    tagline: 'For teams and agencies running many sites.',
    price: '$15',
    priceNote: 'per month',
    annualNote: 'or $120 / year — save 33%',
    ctaLabel: 'Start Business',
    ctaHref: '/register?plan=business',
    features: [
      '30 websites',
      '100,000 page views / day',
      'Every feature included',
      'MCP server, CLI, and API',
      'Priority email support'
    ]
  }
];

type Cell = boolean | string;
export const pricingComparison: { feature: string; selfHosted: Cell; free: Cell; pro: Cell; business: Cell }[] = [
  { feature: 'Websites', selfHosted: 'Unlimited', free: '1', pro: '10', business: '30' },
  { feature: 'Page views per day', selfHosted: 'Unlimited', free: '3,000', pro: '30,000', business: '100,000' },
  { feature: 'Annual price', selfHosted: '$0', free: '$0', pro: '$56', business: '$120' },
  { feature: 'Real-time Spy and visitor details', selfHosted: true, free: true, pro: true, business: true },
  { feature: 'Goals, funnels, and revenue', selfHosted: true, free: true, pro: true, business: true },
  { feature: 'Attribution, campaigns, and journeys', selfHosted: true, free: true, pro: true, business: true },
  { feature: 'MCP server for AI agents', selfHosted: true, free: true, pro: true, business: true },
  { feature: 'CLI, REST API, and CSV export', selfHosted: true, free: true, pro: true, business: true },
  { feature: 'First-party anti-adblock proxy', selfHosted: true, free: true, pro: true, business: true },
  { feature: 'Cookieless, privacy-first tracking', selfHosted: true, free: true, pro: true, business: true },
  { feature: 'Run it on your own servers', selfHosted: true, free: false, pro: false, business: false },
  { feature: 'Managed hosting and backups', selfHosted: false, free: true, pro: true, business: true },
  { feature: 'Support', selfHosted: 'Community', free: 'Community', pro: 'Email', business: 'Priority email' }
];

export const pricingFaqs = [
  {
    question: 'Are any features locked to paid plans?',
    answer:
      'No. Every plan, including Free and self-hosted, gets every feature. Plans differ only in how many sites and page views you track and in the support you get.'
  },
  {
    question: 'How does Slimlytics compare to Clicky?',
    answer:
      'Clicky is mature and has extras Slimlytics does not offer yet, such as heatmaps and uptime monitoring. Slimlytics is open source and self-hostable, built in Rust, includes an MCP server so AI agents can install and query it, and costs less at every tier with no features held back.'
  },
  {
    question: 'What happens if I go over my daily page views?',
    answer:
      'Nothing is dropped. Slimlytics keeps collecting and the dashboard asks you to upgrade. Upgrades, downgrades, and cancellation are self-serve, billed monthly or annually through Stripe.'
  },
  {
    question: 'What is included when I self-host?',
    answer:
      'The complete open-source product: multi-site analytics, the cookieless tracker, real-time Spy, reports, goals, the MCP server, CLI, and first-party proxy setup. You run Docker Compose and own the data.'
  },
  {
    question: 'Do you use cookies or sell visitor data?',
    answer:
      'No. Tracking is cookieless by default, visitor IDs are site-scoped, and we never build cross-site advertising profiles or sell visitor data.'
  },
  {
    question: 'Can I move from hosted to self-hosted later?',
    answer:
      'Yes. Export your data via CSV or the API and run the same open-source stack on your own servers whenever you want full control.'
  }
];
