# Things to fix

--

# Completed

## Duplicate Events

Every time I browse to a new url, the even is being recorded twice for some reason. The Page Views goes up by 2.

If I'm about /about and go to /marketplace, The content shows a hit for /about and /marketplace. It is like it is tracking when I leave a page

- Fixed by simplifying the tracking logic to use a single unified approach with proper debouncing
- Removed conflicting multiple detection methods that were causing duplicate tracking
- Now only tracks the new page when navigation occurs, not the old page

## Recent Visitors

Recent visitors are not showing.

- Fixed by updating the query to use the sessions table's landing_page field directly
- Removed the problematic LEFT JOIN with events table
- Now properly shows recent unique visitors with their landing pages