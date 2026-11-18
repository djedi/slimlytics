# Server-side request ingestion

Server ingestion captures crawlers and non-JavaScript requests from trusted access logs. It uses a separate rotatable site secret and never stores a raw client IP.

## Endpoint

Find the endpoint in **Settings → Server collection** or in the authenticated site response:

```text
POST https://slimlytics.com/api/ingest
Content-Type: application/json
X-Slimlytics-Server-Key: {serverWriteKey}
```

The request body contains 1-100 events:

```json
{
  "events": [
    {
      "idempotencyKey": "edge:request-018f67a3",
      "url": "https://example.com/docs?utm_source=ai",
      "userAgent": "GPTBot/1.0",
      "clientIp": "203.0.113.20",
      "occurredAt": "2026-07-31T18:30:00Z",
      "method": "GET",
      "status": 200,
      "eventName": "pageview"
    }
  ]
}
```

URLs must use the site's configured domain. Only GET/HEAD requests and timestamps from the previous seven days are accepted. The source idempotency key prevents log-shipping retries from creating duplicates. One invalid event rejects the whole batch with `400`; unknown fields are ignored.

The backend uses the IP transiently for GeoIP enrichment and rotating HMAC identifiers, then discards it. It stores sanitized URLs, bounded request metadata, caller properties, UA classification, coarse location, and `ingestionSource=server`.

## Server-confirmed conversions

Ingestion also records custom events your server confirms, such as a signup or a payment, so ad blockers and unloaded pages cannot drop them. Send the visitor's IP and User-Agent from the original request, with two optional fields:

```json
{
  "events": [
    {
      "eventName": "paid",
      "url": "https://example.com/billing",
      "clientIp": "203.0.113.20",
      "userAgent": "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)",
      "idempotencyKey": "paid:invoice-4821",
      "confirmed": true,
      "properties": { "plan": "pro", "revenue": 9.99, "currency": "USD" }
    }
  ]
}
```

- `confirmed` (boolean, default `false`): your server vouches that a visitor did this. Events are normally classified by User-Agent and client network, and an event classified as a bot is stored but excluded from goals and reports. A confirmed event is counted as a visitor action whatever its User-Agent (for example an app's HTTP client or a link-preview string); internal addresses still apply. It is accepted only on custom events: a `pageview`, including one that omits `eventName`, with `confirmed: true` is rejected with `400`, so log-shipped pageviews always keep bot filtering.
- `properties` (object, optional): the same event properties the browser tracker sends, with the same limits: a JSON object of flat values, at most 16 KB serialized. Anything else is rejected with `400` (`properties must be an object` or `event payload too large`). Never send names, emails, or other personal data. A numeric `revenue` (0 to 1,000,000,000,000) is recorded as revenue, with an optional three-letter `currency`; a revenue string such as `"9.99"` stays a plain property. `contentId`, `contentType`, and `contentAuthor` are recorded as content attribution as they are for browser events. Properties appear in event lists and in the recent conversions of the conversions report.

The request `method` and `status`, which ingestion has always stored as properties, take precedence over same-named properties when they are sent.

Honor the same privacy choices as the browser tracker: do not send an event when the visitor has not given analytics consent, or when the original request carried `DNT: 1` or `Sec-GPC: 1`. Server ingestion has no privacy-reduced mode.

## Caddy logs

Configure Caddy JSON access logs, then stream them through the included forwarder:

```bash
export SLIMLYTICS_SERVER_INGEST_URL='https://slimlytics.com/api/ingest'
export SLIMLYTICS_SERVER_WRITE_KEY='YOUR_SERVER_KEY'
export SLIMLYTICS_SITE_ORIGIN='https://example.com'
tail -F /var/log/caddy/access.log | node scripts/server-log-forwarder.mjs
```

The default `SLIMLYTICS_LOG_MODE=bots` forwards only crawler-like user agents. Set `SLIMLYTICS_LOG_MODE=all` only when browser pageview tracking is disabled or deduplicated upstream; otherwise human pageviews will be counted twice.

Run the forwarder under the site's service manager and restrict its environment file because it contains a collection secret. The secret is sent in a header rather than the URL so normal request logs do not capture it. Rotate the key from Settings after suspected exposure.
