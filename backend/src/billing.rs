//! Hosted-plan billing: configurable plans, Stripe webhook verification, and subscription →
//! plan resolution. Disabled by default; self-hosted installs run without limits or Stripe.
//!
//! Configuration (environment):
//! - `BILLING_ENABLED=true` turns plans and limits on.
//! - `BILLING_PLANS_FILE` optionally points at a JSON array of plans (see [`Plan`]); otherwise
//!   the built-in plans below are used.
//! - `STRIPE_SECRET_KEY` (prefer a restricted `rk_` key) and `STRIPE_WEBHOOK_SECRET` enable
//!   Checkout, the Customer Portal, and webhook sync. Stripe prices are referenced by lookup
//!   key, so the same plan file works in a sandbox and in live mode.

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;

/// Plan granted by an administrator for internal or complimentary accounts; no limits.
pub const UNLIMITED_PLAN: &str = "unlimited";
/// Pinned Stripe API version for every request this integration makes.
pub const STRIPE_API_VERSION: &str = "2026-08-26.dahlia";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plan {
    pub id: String,
    pub name: String,
    /// Maximum sites the account may own; `None` is unlimited.
    pub sites: Option<u32>,
    /// Page views per UTC day across the account's sites; `None` is unlimited. Soft limit:
    /// collection continues and the dashboard prompts an upgrade.
    pub daily_page_views: Option<u64>,
    #[serde(default)]
    pub monthly_price_cents: u32,
    #[serde(default)]
    pub annual_price_cents: u32,
    #[serde(default = "usd")]
    pub currency: String,
    /// Stripe Price lookup keys. A plan without them cannot be purchased (e.g. Free).
    #[serde(default, skip_serializing)]
    pub stripe_monthly_lookup_key: Option<String>,
    #[serde(default, skip_serializing)]
    pub stripe_annual_lookup_key: Option<String>,
}

fn usd() -> String {
    "usd".into()
}

impl Plan {
    pub fn purchasable(&self) -> bool {
        self.stripe_monthly_lookup_key.is_some() || self.stripe_annual_lookup_key.is_some()
    }
    pub fn lookup_key(&self, interval: Interval) -> Option<&str> {
        match interval {
            Interval::Month => self.stripe_monthly_lookup_key.as_deref(),
            Interval::Year => self.stripe_annual_lookup_key.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Interval {
    Month,
    Year,
}

#[derive(Debug, Clone)]
pub struct BillingConfig {
    pub plans: Vec<Plan>,
    /// Plan for accounts without a subscription or override (the first plan, normally Free).
    pub default_plan: String,
    pub stripe: Option<StripeConfig>,
}

#[derive(Clone)]
pub struct StripeConfig {
    pub secret_key: String,
    pub webhook_secret: String,
    pub api_base: String,
}

impl std::fmt::Debug for StripeConfig {
    // Never print keys.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StripeConfig")
            .field("api_base", &self.api_base)
            .finish_non_exhaustive()
    }
}

/// The default hosted plans, matching the marketing pricing page.
pub fn default_plans() -> Vec<Plan> {
    let plan = |id: &str, name: &str, sites, views, monthly, annual, keys: bool| Plan {
        id: id.into(),
        name: name.into(),
        sites: Some(sites),
        daily_page_views: Some(views),
        monthly_price_cents: monthly,
        annual_price_cents: annual,
        currency: usd(),
        stripe_monthly_lookup_key: keys.then(|| format!("slimlytics_{id}_monthly")),
        stripe_annual_lookup_key: keys.then(|| format!("slimlytics_{id}_annual")),
    };
    vec![
        plan("free", "Free", 1, 3_000, 0, 0, false),
        plan("pro", "Pro", 10, 30_000, 700, 5_600, true),
        plan("business", "Business", 30, 100_000, 1_500, 12_000, true),
    ]
}

impl BillingConfig {
    pub fn new(plans: Vec<Plan>, stripe: Option<StripeConfig>) -> Result<Self, String> {
        if plans.is_empty() {
            return Err("billing needs at least one plan".into());
        }
        let mut seen = std::collections::HashSet::new();
        for plan in &plans {
            if plan.id.is_empty()
                || !plan
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            {
                return Err(format!("invalid plan id {:?}", plan.id));
            }
            if plan.id == UNLIMITED_PLAN {
                return Err(format!(
                    "plan id {UNLIMITED_PLAN:?} is reserved for admin overrides"
                ));
            }
            if !seen.insert(plan.id.clone()) {
                return Err(format!("duplicate plan id {:?}", plan.id));
            }
        }
        Ok(Self {
            default_plan: plans[0].id.clone(),
            plans,
            stripe,
        })
    }

    pub fn from_json(json: &str, stripe: Option<StripeConfig>) -> Result<Self, String> {
        let plans: Vec<Plan> =
            serde_json::from_str(json).map_err(|e| format!("invalid plans file: {e}"))?;
        Self::new(plans, stripe)
    }

    /// The plan with this id; unknown ids (e.g. a plan removed from config) fall back to the
    /// default, and the admin `unlimited` plan has no limits.
    pub fn plan(&self, id: &str) -> Plan {
        if id == UNLIMITED_PLAN {
            return Plan {
                id: UNLIMITED_PLAN.into(),
                name: "Unlimited".into(),
                sites: None,
                daily_page_views: None,
                monthly_price_cents: 0,
                annual_price_cents: 0,
                currency: usd(),
                stripe_monthly_lookup_key: None,
                stripe_annual_lookup_key: None,
            };
        }
        self.plans
            .iter()
            .find(|plan| plan.id == id)
            .or_else(|| self.plans.iter().find(|plan| plan.id == self.default_plan))
            .cloned()
            .expect("config always has a default plan")
    }

    /// The plan a Stripe subscription grants: matched by price lookup key, and only while the
    /// subscription is in good standing. Anything else falls back to the default plan.
    pub fn plan_for_subscription(&self, subscription: &Value) -> (String, Option<Interval>) {
        let status = subscription["status"].as_str().unwrap_or("");
        if !matches!(status, "active" | "trialing" | "past_due") {
            return (self.default_plan.clone(), None);
        }
        let items = subscription["items"]["data"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for item in items {
            let price = &item["price"];
            if let Some(key) = price["lookup_key"].as_str() {
                for plan in &self.plans {
                    if plan.stripe_monthly_lookup_key.as_deref() == Some(key) {
                        return (plan.id.clone(), Some(Interval::Month));
                    }
                    if plan.stripe_annual_lookup_key.as_deref() == Some(key) {
                        return (plan.id.clone(), Some(Interval::Year));
                    }
                }
            }
            // A superseded price loses its lookup key to the new one, but existing subscribers
            // stay on it; the `slimlytics_plan` metadata set by scripts/stripe-setup.mjs keeps
            // them on their plan.
            if let Some(id) = price["metadata"]["slimlytics_plan"].as_str() {
                if self.plans.iter().any(|plan| plan.id == id) {
                    let interval = match price["recurring"]["interval"].as_str() {
                        Some("year") => Some(Interval::Year),
                        _ => Some(Interval::Month),
                    };
                    return (id.to_owned(), interval);
                }
            }
        }
        (self.default_plan.clone(), None)
    }
}

/// Verifies a `Stripe-Signature` header (`t=…,v1=…[,v1=…]`) against the raw request body,
/// rejecting timestamps outside `tolerance_seconds` of `now` to block replays.
pub fn verify_webhook_signature(
    payload: &[u8],
    header: &str,
    secret: &str,
    now: i64,
    tolerance_seconds: i64,
) -> bool {
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", value)) => timestamp = value.parse::<i64>().ok(),
            Some(("v1", value)) => signatures.push(value.to_owned()),
            _ => {}
        }
    }
    let Some(timestamp) = timestamp else {
        return false;
    };
    if (now - timestamp).abs() > tolerance_seconds || signatures.is_empty() {
        return false;
    }
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload);
    let expected = mac.finalize().into_bytes();
    signatures.iter().any(|signature| {
        hex_decode(signature).is_some_and(|bytes| {
            bytes.len() == expected.len()
                && bytes
                    .iter()
                    .zip(expected.iter())
                    .fold(0u8, |acc, (a, b)| acc | (a ^ b))
                    == 0
        })
    })
}

fn hex_decode(value: &str) -> Option<Vec<u8>> {
    // Works on bytes, so non-ASCII input is rejected instead of slicing mid-character.
    let digit = |b: u8| (b as char).to_digit(16).map(|d| d as u8);
    let bytes = value.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    bytes
        .chunks(2)
        .map(|pair| Some(digit(pair[0])? << 4 | digit(pair[1])?))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sign(payload: &str, secret: &str, t: i64) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(format!("{t}.{payload}").as_bytes());
        let hex: String = mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        format!("t={t},v1={hex}")
    }

    #[test]
    fn verifies_stripe_signatures_and_rejects_tampering_and_replays() {
        let payload = r#"{"id":"evt_1","type":"invoice.paid"}"#;
        let header = sign(payload, "whsec_test", 1_000);
        assert!(verify_webhook_signature(
            payload.as_bytes(),
            &header,
            "whsec_test",
            1_100,
            300
        ));
        assert!(
            !verify_webhook_signature(b"{\"id\":\"evt_2\"}", &header, "whsec_test", 1_100, 300),
            "tampered body"
        );
        assert!(
            !verify_webhook_signature(payload.as_bytes(), &header, "whsec_other", 1_100, 300),
            "wrong secret"
        );
        assert!(
            !verify_webhook_signature(payload.as_bytes(), &header, "whsec_test", 1_400, 300),
            "replayed"
        );
        assert!(
            !verify_webhook_signature(payload.as_bytes(), "v1=abc", "whsec_test", 1_100, 300),
            "no timestamp"
        );
        // Stripe may send several v1 signatures during secret rotation; any valid one passes.
        let rotated = format!("{},v1=deadbeef", sign(payload, "whsec_test", 1_000));
        assert!(verify_webhook_signature(
            payload.as_bytes(),
            &rotated,
            "whsec_test",
            1_000,
            300
        ));
    }

    #[test]
    fn default_plans_match_the_pricing_page() {
        let config = BillingConfig::new(default_plans(), None).unwrap();
        assert_eq!(config.default_plan, "free");
        let pro = config.plan("pro");
        assert_eq!(
            (
                pro.sites,
                pro.daily_page_views,
                pro.monthly_price_cents,
                pro.annual_price_cents
            ),
            (Some(10), Some(30_000), 700, 5_600)
        );
        assert_eq!(
            pro.lookup_key(Interval::Year),
            Some("slimlytics_pro_annual")
        );
        assert!(!config.plan("free").purchasable());
        assert_eq!(
            config.plan("removed-plan").id,
            "free",
            "unknown plans fall back to the default"
        );
        assert_eq!(config.plan(UNLIMITED_PLAN).sites, None);
    }

    /// The documented example config (used by scripts/stripe-setup.mjs) must stay identical
    /// to the built-in defaults.
    #[test]
    fn example_plans_file_matches_the_defaults() {
        let example =
            BillingConfig::from_json(include_str!("../../config/plans.example.json"), None)
                .unwrap();
        assert_eq!(example.plans, default_plans());
    }

    #[test]
    fn non_ascii_signatures_are_rejected_without_panicking() {
        assert_eq!(hex_decode("a€"), None);
        assert_eq!(hex_decode("0aff"), Some(vec![0x0a, 0xff]));
        assert!(!verify_webhook_signature(
            b"{}",
            "t=1000,v1=a€",
            "whsec",
            1000,
            300
        ));
    }

    #[test]
    fn superseded_prices_keep_their_plan_through_metadata() {
        let config = BillingConfig::new(default_plans(), None).unwrap();
        let subscription = json!({"status": "active", "items": {"data": [{"price": {
            "lookup_key": null, "metadata": {"slimlytics_plan": "business"},
            "recurring": {"interval": "year"}
        }}]}});
        assert_eq!(
            config.plan_for_subscription(&subscription),
            ("business".into(), Some(Interval::Year))
        );
        let unknown = json!({"status": "active", "items": {"data": [{"price": {"metadata": {"slimlytics_plan": "gone"}}}]}});
        assert_eq!(config.plan_for_subscription(&unknown).0, "free");
    }

    #[test]
    fn plans_load_from_json_and_reject_bad_config() {
        let config = BillingConfig::from_json(
            r#"[{"id":"hobby","name":"Hobby","sites":2,"dailyPageViews":null,"monthlyPriceCents":300,"stripeMonthlyLookupKey":"hobby_m"}]"#,
            None,
        )
        .unwrap();
        assert_eq!(config.default_plan, "hobby");
        assert_eq!(config.plan("hobby").daily_page_views, None);
        assert!(BillingConfig::from_json("[]", None).is_err());
        assert!(BillingConfig::from_json(
            r#"[{"id":"unlimited","name":"x","sites":1,"dailyPageViews":1}]"#,
            None
        )
        .is_err());
        assert!(BillingConfig::from_json(r#"[{"id":"a","name":"A","sites":1,"dailyPageViews":1},{"id":"a","name":"B","sites":1,"dailyPageViews":1}]"#, None).is_err());
        assert!(BillingConfig::from_json(
            r#"[{"id":"a","name":"A","sites":1,"dailyPageViews":1,"typo":true}]"#,
            None
        )
        .is_err());
    }

    #[test]
    fn subscriptions_map_to_plans_only_in_good_standing() {
        let config = BillingConfig::new(default_plans(), None).unwrap();
        let sub = |status: &str, key: &str| json!({"status": status, "items": {"data": [{"price": {"lookup_key": key}}]}});
        assert_eq!(
            config.plan_for_subscription(&sub("active", "slimlytics_business_annual")),
            ("business".into(), Some(Interval::Year))
        );
        assert_eq!(
            config.plan_for_subscription(&sub("past_due", "slimlytics_pro_monthly")),
            ("pro".into(), Some(Interval::Month))
        );
        assert_eq!(
            config
                .plan_for_subscription(&sub("canceled", "slimlytics_pro_monthly"))
                .0,
            "free"
        );
        assert_eq!(
            config
                .plan_for_subscription(&sub("active", "some_other_product"))
                .0,
            "free"
        );
    }

    #[test]
    fn stripe_config_debug_never_prints_keys() {
        let config = StripeConfig {
            secret_key: "rk_test_secret".into(),
            webhook_secret: "whsec_secret".into(),
            api_base: "https://api.stripe.com".into(),
        };
        let printed = format!("{config:?}");
        assert!(!printed.contains("rk_test_secret") && !printed.contains("whsec_secret"));
    }
}
