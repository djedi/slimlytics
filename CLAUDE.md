# Slim Analytics

The goal of this project is to create a small, open source analytics app. I find Google Analytics too complicated. I like Clicky, but I don't want to pay to track my 10 visitors per day on 10 different websites.

So I want a small, fun, easy to read analytics app that can run in docker on a $5 digital ocean droplet.

## Tech Stack

- Bun: Because it is JavaScript and it is fast - surprisingly close to Go when it comes to performance
- SQLite: Has more power than we give it credit for. Small footprint, easy to back up, and quite sufficient for this use case.
- 11ty: A simple static site generator used to build the dashboard with minimal overhead.
- HTMX: Enables server-driven UI updates with minimal JavaScript by handling AJAX, CSS transitions, and more.
- Alpine.js: Provides lightweight client-side interactivity and state management to enhance the user experience.

## Architecture

The system uses a Bun-based ingest API to efficiently receive tracking data, which is stored in SQLite for lightweight and reliable persistence. The dashboard is built as a static site using 11ty, with HTMX and Alpine.js enabling server-driven UI updates and small client-side enhancements for a responsive and dynamic interface without heavy frontend frameworks.

## Features

- Lightweight tracking optimized for low-traffic sites.
- Batched inserts to SQLite to reduce write overhead.
- Privacy-friendly design avoiding invasive tracking techniques.
- Daily rollups of analytics data for efficient reporting.
- Retention strategy to manage data size and relevance over time.

## Why

This stack is ideal for small websites running on a $5 DigitalOcean droplet because it combines simplicity, performance, and minimal resource usage. Bun and SQLite provide fast and efficient backend processing, while 11ty, HTMX, and Alpine.js deliver a dynamic yet lightweight frontend without the complexity of heavier frameworks.

## Coding Style

- Put CSS and JS in their own files rather than mixing with html
