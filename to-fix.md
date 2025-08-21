# Things to fix

## Visitors Graph

It is not rendering. It is just blank.

## Duplicate Events

Every time I browse to a new url, the even is being recorded twice for some reason. The Page Views goes up by 2.

## Site Selector

The current site is unreadable in the Site Selector because it is white on a light gray background

## Site Statistics

On the settings page there are site statistics with placeholder data. Put real data there.

## Tracking Code

Remove links for "Plugins"

Implement "Verify Tracking Code" that will check the site to see if the tracking code is working.

Remove Affiliate badge checkbox

## Settings

Remove Tracking code section from the settings as there is a separate view for this

## Test

Update test view to test with session data

## Sample Data

Don't load sample data if data is not loading. It is confusing.

--

# Completed

## Recent Visitors

Should be unique visitors and the path they landed on, not events.

It should display time, locale flag, IP (not masked), path landed on (linked to url)

Example:

- 7:26a 🇺🇸 216.41.234.0 /blog/envelope-budgeting-basics/
- 7:26a 🇺🇸 68.234.46.0 /blog/markdown-reports-ai-coaching/
- 6:49a 🇳🇱 149.57.191.0 /
- 1:20a 🇦🇹 185.50.234.0 /

## Clear Data

In Settings there is a button to clear all data. This no longer works as it doesn't clear session data.
