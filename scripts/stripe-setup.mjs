#!/usr/bin/env node
// Create (or verify) the Stripe Products and Prices that Slimlytics plans reference by lookup
// key. Safe to re-run: existing prices with matching amounts are left alone, and changed amounts
// get a new Price that takes over the lookup key (Stripe prices are immutable).
//
// Usage:
//   STRIPE_SECRET_KEY=rk_test_... node scripts/stripe-setup.mjs            # dry run
//   STRIPE_SECRET_KEY=rk_test_... node scripts/stripe-setup.mjs --apply    # create/update
//   ... --plans path/to/plans.json   (default: BILLING_PLANS_FILE or config/plans.example.json)
//   ... --live                       (required to use a live-mode key)
//
// The key needs write access to Products and Prices. Prefer a restricted key (rk_).
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const API = 'https://api.stripe.com';
const VERSION = '2026-08-26.dahlia';
const args = process.argv.slice(2);
const apply = args.includes('--apply');
const live = args.includes('--live');
const plansPath =
  args[args.indexOf('--plans') + 1] && args.includes('--plans')
    ? args[args.indexOf('--plans') + 1]
    : process.env.BILLING_PLANS_FILE || fileURLToPath(new URL('../config/plans.example.json', import.meta.url));

const key = process.env.STRIPE_SECRET_KEY;
if (!key) fail('Set STRIPE_SECRET_KEY (a restricted rk_ key with Products and Prices write access).');
if (/^[sr]k_live_/.test(key) && !live) fail('Refusing to use a live-mode key without --live.');

const plans = JSON.parse(readFileSync(plansPath, 'utf8'));
console.log(`${apply ? 'Applying' : 'Dry run for'} ${plansPath} (${/_live_/.test(key) ? 'LIVE' : 'test'} mode)`);

for (const plan of plans) {
  const variants = [
    ['month', plan.stripeMonthlyLookupKey, plan.monthlyPriceCents],
    ['year', plan.stripeAnnualLookupKey, plan.annualPriceCents]
  ].filter(([, lookupKey]) => lookupKey);
  if (!variants.length) {
    console.log(`- ${plan.id}: not purchasable (no lookup keys), skipped`);
    continue;
  }
  // One Product per plan; monthly and annual are Prices on it.
  let product = (await stripe('GET', '/v1/products/search', { query: `metadata['slimlytics_plan']:'${plan.id}'` })).data[0];
  for (const [interval, lookupKey, amount] of variants) {
    const existing = (await stripe('GET', '/v1/prices', { 'lookup_keys[]': lookupKey, active: 'true', limit: '1' })).data[0];
    if (existing && existing.unit_amount === amount && existing.currency === (plan.currency || 'usd')) {
      if (existing.metadata?.slimlytics_plan === plan.id) {
        console.log(`- ${lookupKey}: ok (${existing.id})`);
      } else {
        console.log(`- ${lookupKey}: ok (${existing.id}); will tag metadata slimlytics_plan=${plan.id}`);
        if (apply) await stripe('POST', `/v1/prices/${existing.id}`, { 'metadata[slimlytics_plan]': plan.id });
      }
      continue;
    }
    console.log(`- ${lookupKey}: ${existing ? `amount changed ${existing.unit_amount} → ${amount}` : 'missing'}; will create ${amount} ${plan.currency || 'usd'}/${interval}`);
    if (!apply) continue;
    if (!product) {
      product = await stripe('POST', '/v1/products', { name: `Slimlytics ${plan.name}`, 'metadata[slimlytics_plan]': plan.id });
      console.log(`  created product ${product.id}`);
    }
    const price = await stripe('POST', '/v1/prices', {
      product: product.id,
      unit_amount: String(amount),
      currency: plan.currency || 'usd',
      'recurring[interval]': interval,
      lookup_key: lookupKey,
      transfer_lookup_key: 'true',
      // Old prices lose the lookup key when amounts change; this keeps their subscribers mapped.
      'metadata[slimlytics_plan]': plan.id,
      nickname: `${plan.name} ${interval === 'month' ? 'monthly' : 'annual'}`
    });
    console.log(`  created price ${price.id}`);
  }
}
if (!apply) console.log('Dry run only. Re-run with --apply to make these changes.');

async function stripe(method, path, params) {
  const body = new URLSearchParams(params);
  const url = method === 'GET' ? `${API}${path}?${body}` : `${API}${path}`;
  const response = await fetch(url, {
    method,
    headers: { authorization: `Bearer ${key}`, 'stripe-version': VERSION, 'content-type': 'application/x-www-form-urlencoded' },
    body: method === 'GET' ? undefined : body
  });
  const json = await response.json();
  if (!response.ok) fail(`Stripe ${method} ${path}: ${json.error?.message ?? response.status}`);
  return json;
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
