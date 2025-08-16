#!/bin/bash

# Kill any existing processes on ports 3000 and 8080
echo "Checking for existing services..."

# Kill process on port 3000 (API server)
PID_3000=$(lsof -ti:3000)
if [ ! -z "$PID_3000" ]; then
    echo "Killing process on port 3000 (PID: $PID_3000)"
    kill -9 $PID_3000 2>/dev/null
    sleep 1
fi

# Kill process on port 8080 (Eleventy dev server)
PID_8080=$(lsof -ti:8080)
if [ ! -z "$PID_8080" ]; then
    echo "Killing process on port 8080 (PID: $PID_8080)"
    kill -9 $PID_8080 2>/dev/null
    sleep 1
fi

# Also kill any running bun or eleventy processes
pkill -f "bun.*server.js" 2>/dev/null
pkill -f "eleventy.*serve" 2>/dev/null

echo "Starting development servers..."

# Start the development servers
exec concurrently "bun run dev:api" "bun run dev:dashboard"