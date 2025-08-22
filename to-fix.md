# Things to fix

--

# Completed

## Duplicate Events

Every time I browse to a new url, the even is being recorded twice for some reason. The Page Views goes up by 2.

If I'm about /about and go to /marketplace, The content shows a hit for /about and /marketplace. It is like it is tracking when I leave a page

- Fixed by simplifying the tracking logic to use a single unified approach with proper debouncing
- Added pending track timeout to prevent multiple events
- Now only tracks the new page when navigation occurs, not the old page

## Recent Visitors

Recent visitors are not showing.

- Fixed by updating the SQL query to join with events table properly
- Changed from non-existent `landing_page` column to using `MIN(e.page_url)` from events
- Now properly shows recent unique visitors with their first visited page

## Note on Deployment

The fixes have been implemented in the source code but require deployment to production for the changes to take effect. The test showed events are being tracked successfully, but the dashboard cannot display them due to the SQL error in the current production code.