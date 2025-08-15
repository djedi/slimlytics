#!/bin/bash

echo "🚀 Starting Slimlytics..."

# Create data directory if it doesn't exist
mkdir -p data

# Initialize database
echo "📊 Initializing database..."
bun run src/db/init.js

# Start the API server in background
echo "🔧 Starting API server on port 3000..."
bun run src/api/server.js &
API_PID=$!

# Start the dashboard server
echo "🌐 Starting dashboard on port 8080..."
npx @11ty/eleventy --serve --port=8080 &
DASHBOARD_PID=$!

echo "✅ Slimlytics is running!"
echo "   API: http://localhost:3000"
echo "   Dashboard: http://localhost:8080"
echo ""
echo "Press Ctrl+C to stop..."

# Wait for interrupt signal
trap "echo '🛑 Shutting down...'; kill $API_PID $DASHBOARD_PID; exit" INT

# Keep script running
wait