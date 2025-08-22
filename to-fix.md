# Things to fix

--

# Completed

## Duplicate Events

Every time I browse to a new url, the even is being recorded twice for some reason. The Page Views goes up by 2.

If I'm about /about and go to /marketplace, The content shows a hit for /about and /marketplace. It is like it is tracking when I leave a page

- Fixed by using full URL comparison instead of just pathname to prevent duplicate tracking

## Visitors Graph

It is not rendering. It is just blank - only on the Today view though. If I switch to another day or period it renders.

- Fixed by handling single data point scenarios by duplicating the point to allow chart rendering

## Site Selector

The current site is unreadable in the Site Selector because it is white on a light gray background. It is the `button` css that is setting it white.

- Fixed by explicitly setting color: #212529 on .site-selector-btn to override global button style

## Recent Visitors

Recent visitors should show country flag associated with IP address like so:

- 7:26a 🇺🇸 216.41.234.0 /blog/envelope-budgeting-basics/
- 7:26a 🇺🇸 68.234.46.0 /blog/markdown-reports-ai-coaching/
- 6:49a 🇳🇱 149.57.191.0 /
- 1:20a 🇦🇹 185.50.234.0 /

- Already implemented correctly - shows time, flag emoji, IP address, and path

## Tracking Code

Tracking code is not being displayed so I can't copy it to clipboard.

- Fixed by:
  1. Properly handling siteData initialization
  2. Correcting script reference from /sa.js to /t.js
  3. Using correct data-site attribute instead of data-id