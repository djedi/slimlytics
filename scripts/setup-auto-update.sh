#!/bin/bash

# Setup script for automatic MaxMind database updates
# This script sets up a cron job or systemd timer to automatically update GeoIP databases

echo "=== Slimlytics MaxMind Auto-Update Setup ==="
echo ""

# Check if running as root for systemd setup
IS_ROOT=0
if [ "$EUID" -eq 0 ]; then 
    IS_ROOT=1
fi

# Get the Slimlytics directory
SLIMLYTICS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
echo "Slimlytics directory: $SLIMLYTICS_DIR"
echo ""

# Function to setup cron job
setup_cron() {
    echo "Setting up cron job for automatic updates..."
    
    # Create the cron command
    CRON_CMD="cd $SLIMLYTICS_DIR && /usr/local/bin/bun run maxmind:update >> $SLIMLYTICS_DIR/data/maxmind/cron.log 2>&1"
    
    # Add to crontab (runs every Wednesday and Saturday at 3 AM)
    (crontab -l 2>/dev/null; echo "0 3 * * 3,6 $CRON_CMD") | crontab -
    
    echo "✅ Cron job added successfully!"
    echo "   Updates will run every Wednesday and Saturday at 3:00 AM"
    echo ""
    echo "To view your cron jobs, run: crontab -l"
    echo "To remove the cron job, run: crontab -e and delete the line"
}

# Function to setup systemd timer (for production servers)
setup_systemd() {
    if [ $IS_ROOT -eq 0 ]; then
        echo "❌ Systemd setup requires root privileges."
        echo "   Please run: sudo $0"
        exit 1
    fi
    
    echo "Setting up systemd timer for automatic updates..."
    
    # Create systemd service file
    cat > /etc/systemd/system/slimlytics-maxmind-update.service << EOF
[Unit]
Description=Update Slimlytics MaxMind GeoIP Databases
After=network.target

[Service]
Type=oneshot
WorkingDirectory=$SLIMLYTICS_DIR
ExecStart=/usr/local/bin/bun run maxmind:update
User=$SUDO_USER
Group=$SUDO_USER
StandardOutput=append:$SLIMLYTICS_DIR/data/maxmind/systemd.log
StandardError=append:$SLIMLYTICS_DIR/data/maxmind/systemd.log

[Install]
WantedBy=multi-user.target
EOF

    # Create systemd timer file
    cat > /etc/systemd/system/slimlytics-maxmind-update.timer << EOF
[Unit]
Description=Update Slimlytics MaxMind databases twice weekly
Requires=slimlytics-maxmind-update.service

[Timer]
# Run every Wednesday and Saturday at 3 AM
OnCalendar=Wed,Sat 03:00:00
Persistent=true

[Install]
WantedBy=timers.target
EOF

    # Reload systemd and enable timer
    systemctl daemon-reload
    systemctl enable slimlytics-maxmind-update.timer
    systemctl start slimlytics-maxmind-update.timer
    
    echo "✅ Systemd timer created and started!"
    echo ""
    echo "Useful commands:"
    echo "  View timer status:  systemctl status slimlytics-maxmind-update.timer"
    echo "  View next run time: systemctl list-timers slimlytics-maxmind-update"
    echo "  Run update now:     systemctl start slimlytics-maxmind-update.service"
    echo "  View update logs:   journalctl -u slimlytics-maxmind-update.service"
}

# Function to setup launchd (macOS)
setup_launchd() {
    echo "Setting up launchd job for automatic updates (macOS)..."
    
    PLIST_FILE="$HOME/Library/LaunchAgents/com.slimlytics.maxmind-update.plist"
    
    cat > "$PLIST_FILE" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.slimlytics.maxmind-update</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/bun</string>
        <string>run</string>
        <string>maxmind:update</string>
    </array>
    <key>WorkingDirectory</key>
    <string>$SLIMLYTICS_DIR</string>
    <key>StartCalendarInterval</key>
    <array>
        <dict>
            <key>Weekday</key>
            <integer>3</integer>
            <key>Hour</key>
            <integer>3</integer>
            <key>Minute</key>
            <integer>0</integer>
        </dict>
        <dict>
            <key>Weekday</key>
            <integer>6</integer>
            <key>Hour</key>
            <integer>3</integer>
            <key>Minute</key>
            <integer>0</integer>
        </dict>
    </array>
    <key>StandardOutPath</key>
    <string>$SLIMLYTICS_DIR/data/maxmind/launchd.log</string>
    <key>StandardErrorPath</key>
    <string>$SLIMLYTICS_DIR/data/maxmind/launchd-error.log</string>
</dict>
</plist>
EOF

    # Load the launch agent
    launchctl load "$PLIST_FILE"
    
    echo "✅ LaunchAgent created and loaded!"
    echo "   Updates will run every Wednesday and Saturday at 3:00 AM"
    echo ""
    echo "To view status: launchctl list | grep slimlytics"
    echo "To unload: launchctl unload $PLIST_FILE"
}

# Detect OS and offer appropriate setup
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    echo "Detected macOS system"
    echo ""
    echo "Choose setup method:"
    echo "1) Cron (simple, traditional)"
    echo "2) LaunchAgent (macOS native)"
    echo ""
    read -p "Enter choice (1-2): " choice
    
    case $choice in
        1)
            setup_cron
            ;;
        2)
            setup_launchd
            ;;
        *)
            echo "Invalid choice"
            exit 1
            ;;
    esac
else
    # Linux
    echo "Detected Linux system"
    echo ""
    echo "Choose setup method:"
    echo "1) Cron (simple, works everywhere)"
    echo "2) Systemd timer (recommended for production servers)"
    echo ""
    read -p "Enter choice (1-2): " choice
    
    case $choice in
        1)
            setup_cron
            ;;
        2)
            setup_systemd
            ;;
        *)
            echo "Invalid choice"
            exit 1
            ;;
    esac
fi

echo ""
echo "🎉 Auto-update setup complete!"
echo ""
echo "The MaxMind databases will be automatically checked and updated twice weekly."
echo "Updates occur on Wednesdays and Saturdays to catch MaxMind's Tuesday and Friday releases."
echo ""
echo "To manually trigger an update, run: bun run maxmind:update"
echo "To force an update even if databases are current: bun run maxmind:update -- --force"