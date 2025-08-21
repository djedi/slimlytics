# Things to fix

--

# Completed

## Visitors Graph

It is not rendering. It is just blank.
- Fixed by adding $nextTick() to ensure Alpine.js is ready before rendering chart

## Duplicate Events

Every time I browse to a new url, the even is being recorded twice for some reason. The Page Views goes up by 2.
- Fixed by adding debounce mechanism to prevent duplicate tracking within 500ms

## Site Selector

The current site is unreadable in the Site Selector because it is white on a light gray background
- Fixed by adding !important to override conflicting CSS rules

## Site Statistics

On the settings page there are site statistics with placeholder data. Put real data there.
- Fixed by fetching real statistics from the API instead of using hardcoded mock data

## Tracking Code

Remove links for "Plugins"
- Removed "Plugins" link and replaced with verify button

Implement "Verify Tracking Code" that will check the site to see if the tracking code is working.
- Implemented verification function that checks if tracking code is installed

Remove Affiliate badge checkbox
- Removed from HTML and JavaScript

## Settings

Remove Tracking code section from the settings as there is a separate view for this
- Removed tracking code section and related functions from settings page

## Test

Update test view to test with session data
- Added realistic session simulation with consistent user agents and multiple page views per visitor
- Added "Send Realistic Session" button to test sessions

## Sample Data

Don't load sample data if data is not loading. It is confusing.
- Fixed by showing error state instead of loading mock data when API fails

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