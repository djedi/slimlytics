import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import PlanCard from '../src/lib/components/billing/PlanCard.svelte';
import type { BillingStatus } from '../src/lib/api';

const plans = [
  { id: 'free', name: 'Free', sites: 1, dailyPageViews: 3000, monthlyPriceCents: 0, annualPriceCents: 0, currency: 'usd' },
  { id: 'pro', name: 'Pro', sites: 10, dailyPageViews: 30000, monthlyPriceCents: 700, annualPriceCents: 5600, currency: 'usd', intervals: ['month', 'year'] as ('month' | 'year')[] },
  { id: 'business', name: 'Business', sites: 30, dailyPageViews: 100000, monthlyPriceCents: 1500, annualPriceCents: 12000, currency: 'usd', intervals: ['month', 'year'] as ('month' | 'year')[] }
];
const status = (overrides: Partial<BillingStatus>): BillingStatus => ({
  enabled: true,
  plan: plans[0],
  planSource: 'default',
  usage: { sites: 1, pageViewsToday: 1200 },
  plans,
  checkoutAvailable: true,
  hasBillingAccount: false,
  ...overrides
});

describe('plan card', () => {
  afterEach(cleanup);

  it('shows usage against limits and offers upgrades to higher plans', async () => {
    const onCheckout = vi.fn();
    render(PlanCard, { props: { status: status({}), onCheckout, onPortal: vi.fn() } });
    expect(document.body).toHaveTextContent('Free plan');
    expect(document.body).toHaveTextContent('1 of 1 sites');
    expect(document.body).toHaveTextContent('1,200 of 3,000 page views today');
    await fireEvent.click(screen.getByRole('button', { name: /Upgrade to Pro/ }));
    expect(onCheckout).toHaveBeenCalledWith('pro', 'month');
    expect(screen.queryByRole('button', { name: /Upgrade to Free/ })).toBeNull();
  });

  it('keeps Manage billing for a comped account that still has a Stripe customer', () => {
    render(PlanCard, { props: { status: status({ planSource: 'admin', hasBillingAccount: true }), onCheckout: vi.fn(), onPortal: vi.fn() } });
    expect(screen.getByRole('button', { name: /Manage billing/ })).toBeInTheDocument();
  });

  it('offers annual-only plans under the annual toggle', async () => {
    const annualOnly = [plans[0], { ...plans[1], monthlyPriceCents: 0, intervals: ['year'] as ('month' | 'year')[] }];
    render(PlanCard, { props: { status: status({ plans: annualOnly }), onCheckout: vi.fn(), onPortal: vi.fn() } });
    expect(screen.queryByRole('button', { name: /Upgrade to Pro/ })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: /Annual/ }));
    expect(screen.getByRole('button', { name: /Upgrade to Pro/ })).toHaveTextContent('$56/yr');
  });

  it('offers plans that only raise the site allowance', () => {
    const sitesOnly = [
      { ...plans[0], dailyPageViews: null },
      { ...plans[1], dailyPageViews: null }
    ];
    render(PlanCard, { props: { status: status({ plans: sitesOnly, plan: sitesOnly[0] }), onCheckout: vi.fn(), onPortal: vi.fn() } });
    expect(screen.getByRole('button', { name: /Upgrade to Pro/ })).toBeInTheDocument();
  });

  it('prices upgrades in the plan currency', () => {
    const euro = plans.map((plan) => ({ ...plan, currency: 'eur' }));
    render(PlanCard, { props: { status: status({ plans: euro, plan: euro[0] }), onCheckout: vi.fn(), onPortal: vi.fn() } });
    expect(screen.getByRole('button', { name: /Upgrade to Pro/ })).toHaveTextContent('€7');
  });

  it('warns when today is over the soft page-view limit without blocking collection', () => {
    render(PlanCard, { props: { status: status({ usage: { sites: 1, pageViewsToday: 4500 } }), onCheckout: vi.fn(), onPortal: vi.fn() } });
    expect(screen.getByRole('status')).toHaveTextContent(/over today’s limit.*still being collected/i);
  });

  it('lets subscribers manage billing and shows comped plans plainly', () => {
    const onPortal = vi.fn();
    render(PlanCard, { props: { status: status({ plan: plans[1], planSource: 'stripe', hasBillingAccount: true, interval: 'year' }), onCheckout: vi.fn(), onPortal } });
    screen.getByRole('button', { name: 'Manage billing' }).click();
    expect(onPortal).toHaveBeenCalled();
    cleanup();
    render(PlanCard, { props: { status: status({ plan: { ...plans[2], id: 'unlimited', name: 'Unlimited', sites: null, dailyPageViews: null }, planSource: 'admin' }), onCheckout: vi.fn(), onPortal: vi.fn() } });
    expect(document.body).toHaveTextContent('Complimentary');
    expect(document.body).toHaveTextContent('Unlimited sites');
    expect(screen.queryByRole('button', { name: /Upgrade/ })).toBeNull();
  });
});
