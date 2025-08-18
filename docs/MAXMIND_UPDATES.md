# MaxMind Database Auto-Update Guide

## Overview

MaxMind updates their GeoLite2 databases twice weekly:
- **Tuesdays**: New data release
- **Fridays**: New data release

With your account, you can download databases up to **30 times per day**, so we can easily keep your data current without hitting limits.

## Quick Setup

### 1. Manual Update (On-Demand)

Check if databases need updating and download if necessary:
```bash
bun run maxmind:update
```

Force an update even if databases appear current:
```bash
bun run maxmind:update -- --force
```

### 2. Automatic Updates (Recommended)

Set up automatic updates that run twice weekly:
```bash
bun run maxmind:setup-auto-update
```

This interactive script will:
- Detect your operating system
- Offer appropriate scheduling options (cron, systemd, or launchd)
- Configure automatic updates for Wednesdays and Saturdays at 3 AM

## Update Methods

### Method 1: Cron Job (Universal)

Runs on all Unix-like systems (Linux, macOS, etc.)

**Setup:**
```bash
# Add to your crontab
crontab -e

# Add this line (adjust path as needed):
0 3 * * 3,6 cd /path/to/slimlytics && bun run maxmind:update
```

**Benefits:**
- Simple and reliable
- Works on any Unix system
- Easy to modify schedule

### Method 2: Systemd Timer (Linux Production)

Best for production Linux servers.

**Setup:**
```bash
# Run the setup script as root
sudo ./scripts/setup-auto-update.sh
# Choose option 2 (systemd)
```

**Benefits:**
- Integrated with system logging
- Automatic restart on failure
- Better for production environments

**Management commands:**
```bash
# Check timer status
systemctl status slimlytics-maxmind-update.timer

# View next scheduled run
systemctl list-timers slimlytics-maxmind-update

# Run update immediately
sudo systemctl start slimlytics-maxmind-update.service

# View logs
journalctl -u slimlytics-maxmind-update.service
```

### Method 3: LaunchAgent (macOS)

Native macOS scheduling system.

**Setup:**
```bash
# Run the setup script
./scripts/setup-auto-update.sh
# Choose option 2 (LaunchAgent)
```

**Benefits:**
- Native to macOS
- Survives system reboots
- Integrated with macOS logging

### Method 4: Docker/Container Environments

Add to your docker-compose.yml:
```yaml
services:
  maxmind-updater:
    image: node:alpine
    volumes:
      - ./:/app
      - ./data/maxmind:/data/maxmind
    working_dir: /app
    environment:
      - MAXMIND_ACCOUNT_ID=${MAXMIND_ACCOUNT_ID}
      - MAXMIND_LICENSE_KEY=${MAXMIND_LICENSE_KEY}
    command: sh -c "apk add --no-cache dcron && echo '0 3 * * 3,6 cd /app && bun run maxmind:update' | crontab - && crond -f"
```

### Method 5: CI/CD Pipeline

GitHub Actions example (.github/workflows/update-maxmind.yml):
```yaml
name: Update MaxMind Databases

on:
  schedule:
    # Runs at 3 AM UTC on Wednesdays and Saturdays
    - cron: '0 3 * * 3,6'
  workflow_dispatch: # Allow manual trigger

jobs:
  update:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - uses: oven-sh/setup-bun@v1
        with:
          bun-version: latest
      
      - name: Update MaxMind databases
        env:
          MAXMIND_ACCOUNT_ID: ${{ secrets.MAXMIND_ACCOUNT_ID }}
          MAXMIND_LICENSE_KEY: ${{ secrets.MAXMIND_LICENSE_KEY }}
        run: |
          bun install
          bun run maxmind:update
      
      - name: Commit updated databases
        run: |
          git config --local user.email "action@github.com"
          git config --local user.name "GitHub Action"
          git add data/maxmind/version.json
          git diff --staged --quiet || git commit -m "Update MaxMind database version info"
          git push
```

## Update Script Features

The `update-maxmind.js` script includes:

### Smart Update Detection
- Only downloads when databases are missing or >3 days old
- Compares file sizes to detect actual changes
- Tracks update history and statistics

### Version Tracking
Creates `data/maxmind/version.json` with:
- Last update timestamp
- Database file sizes
- Update count

### Logging
Maintains `data/maxmind/update.log` with:
- Update attempts
- Success/failure status
- Error messages

### Production Support
- Automatically restarts API server after updates (systemd/pm2)
- Minimal resource usage
- Silent operation suitable for cron

## Monitoring Updates

### Check Update Status
```bash
# View last update and statistics
cat data/maxmind/version.json | jq '.'

# View update history
tail -20 data/maxmind/update.log

# Check database file dates
ls -la data/maxmind/*.mmdb
```

### Set Up Alerts

Add to your monitoring system:
```bash
#!/bin/bash
# Check if databases are older than 7 days
find data/maxmind -name "*.mmdb" -mtime +7 | grep -q . && \
  echo "WARNING: MaxMind databases are outdated!"
```

## Troubleshooting

### Common Issues

**1. Downloads failing:**
- Check credentials in `.env`
- Verify internet connectivity
- Check MaxMind account status

**2. Cron not running:**
- Verify cron service is running: `service cron status`
- Check cron logs: `/var/log/cron.log` or `grep CRON /var/log/syslog`
- Ensure full paths are used in crontab

**3. Systemd timer not triggering:**
```bash
# Check timer status
systemctl status slimlytics-maxmind-update.timer

# Check service logs
journalctl -u slimlytics-maxmind-update.service -n 50
```

**4. Permission issues:**
- Ensure data/maxmind directory is writable
- Check file ownership matches the user running updates

### Manual Recovery

If automatic updates fail:
```bash
# Force download fresh databases
bun run maxmind:download

# Verify databases are valid
ls -la data/maxmind/*.mmdb

# Restart API server to load new databases
pm2 restart slimlytics  # or systemctl restart slimlytics
```

## Best Practices

1. **Schedule Updates Wisely**
   - Run on Wednesdays and Saturdays (after MaxMind's Tuesday/Friday releases)
   - Choose low-traffic hours (3-5 AM)
   - Avoid running during backup windows

2. **Monitor Database Age**
   - Set up alerts for databases older than 7 days
   - Check update logs weekly
   - Verify databases are actually being updated (check file sizes)

3. **Handle Failures Gracefully**
   - The update script won't restart services if updates fail
   - Old databases continue working if updates fail
   - Check logs regularly for silent failures

4. **Security Considerations**
   - Keep `.env` file secure (chmod 600)
   - Don't commit databases to version control
   - Rotate MaxMind license keys periodically

5. **Resource Management**
   - Updates use ~80MB bandwidth per run
   - Temporary disk space needed: ~100MB
   - CPU usage is minimal (mainly extraction)

## Update Frequency Recommendations

Based on your use case:

- **High-traffic production**: Update twice weekly (Wed/Sat)
- **Medium-traffic**: Update weekly
- **Development/testing**: Update monthly or on-demand
- **Geographic accuracy critical**: Update twice weekly + monitor for special releases

Remember: You have 30 downloads/day available, so don't hesitate to update when needed!