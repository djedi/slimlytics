# Billing

Billing is **off by default**. A self-hosted Slimlytics has no plans, no limits and no Stripe
dependency. Turning it on adds plan limits and Stripe subscriptions, which is how the hosted
service at slimlytics.com runs.

## What a plan limits

| Plan | Sites | Page views/day | Price |
| --- | --- | --- | --- |
| Free | 1 | 3,000 | $0 |
| Pro | 10 | 30,000 | $7/mo or $56/yr |
| Business | 30 | 100,000 | $15/mo or $120/yr |

Every feature is on every plan.

- **Sites** are enforced: creating a site past the limit returns `402 plan_limit`.
- **Daily page views** are a soft limit. Collection never stops. The dashboard shows usage
  and asks the account to upgrade at 80% and 100%.

## Configuration

| Variable | Purpose |
| --- | --- |
| `BILLING_ENABLED` | `true` turns on plans and limits. |
| `BILLING_PLANS_FILE` | Optional JSON plan list. Without it, the defaults above apply ([`config/plans.example.json`](../config/plans.example.json)). |
| `STRIPE_SECRET_KEY` | Stripe API key. Use a restricted key (`rk_`). |
| `STRIPE_WEBHOOK_SECRET` | Signing secret of the webhook endpoint (`whsec_`). |

Without the Stripe variables, limits still apply but checkout is unavailable. Plans can then be
assigned only by an administrator (see below). Setting just one of the two Stripe variables is
a startup error.

Each plan in the file has these fields:

- `id`, `name`
- `sites` and `dailyPageViews` (`null` means unlimited)
- `monthlyPriceCents`, `annualPriceCents`, `currency`
- optional `stripeMonthlyLookupKey` and `stripeAnnualLookupKey`

A plan without lookup keys can't be bought. The first plan is the default for new accounts.
The id `unlimited` is reserved for admin grants.

## Restricted key permissions

The runtime key also reads Customers, to recover an account link from the customer's metadata.

- Customers: write
- Checkout Sessions: write
- Customer portal: write
- Subscriptions: read
- Prices: read
- Products and Prices: write (only for the setup script; it can use a separate key)

## Stripe setup

1. Create the Products and Prices. One Product per plan; each interval is a Price with the plan's lookup key:

   ```sh
   STRIPE_SECRET_KEY=rk_test_... node scripts/stripe-setup.mjs          # dry run
   STRIPE_SECRET_KEY=rk_test_... node scripts/stripe-setup.mjs --apply
   ```

   Live keys also need `--live`. Re-running is safe. When an amount changes, the script creates
   a new Price and moves the lookup key to it, so existing subscribers keep their old price.
2. Add a webhook endpoint at `https://<your-domain>/api/billing/webhook` for these events:
   - `checkout.session.completed`
   - `checkout.session.async_payment_succeeded`
   - `customer.subscription.created`
   - `customer.subscription.updated`
   - `customer.subscription.deleted`
   - `invoice.paid`
   - `invoice.payment_failed`

   Put its signing secret in `STRIPE_WEBHOOK_SECRET`.
3. Turn on the Customer Portal in the Dashboard, including plan switching and cancellation.
4. Optional: Stripe Tax. Checkout doesn't enable `automatic_tax`. If you add it, you need an
   active tax registration first, or Stripe collects no tax.

For local development, forward events with the Stripe CLI:

```sh
stripe listen --forward-to localhost:8080/api/billing/webhook   # prints the whsec_ to use
```

## How it works

- **Checkout:** `/pricing` links to `/register?plan=pro&interval=year`. After sign-up the user
  goes straight to Stripe Checkout. Signed-in users upgrade from the plan card on the dashboard.
- **Webhooks:** the server checks every webhook's signature and drops duplicate deliveries. It
  then re-reads the customer's subscriptions from Stripe, so it doesn't rely on event order.
  The plan comes from the subscription's price lookup key, and only `active`, `trialing` and
  `past_due` subscriptions count.
- **Access:** billing endpoints accept browser sessions only. API and agent tokens get `403`.

## Admin grants

Comp an account, or release it back to Stripe-managed billing:

```sh
scripts/billing-grant.sh you@example.com unlimited   # or business, pro, ...
scripts/billing-grant.sh you@example.com --release
```

Webhooks never overwrite a granted plan.
