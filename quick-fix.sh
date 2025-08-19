#!/bin/bash
# Quick deployment script for production fixes

echo "🚀 Quick Fix Deployment"
echo "This will:"
echo "1. Build and push updated Docker images"
echo "2. Update Caddyfile with CSP fix"
echo ""
read -p "Continue? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]
then
    exit 1
fi

# Build and push the app image with fixes
echo "▸ Building and pushing app image..."
docker buildx build --platform linux/amd64,linux/arm64 --target app -t xhenxhe/slimlytics:app --push . || exit 1

echo "▸ Building and pushing caddy image..."
docker buildx build --platform linux/amd64,linux/arm64 --target caddy -t xhenxhe/slimlytics:caddy --push . || exit 1

echo "✓ Docker images updated"
echo ""
echo "Now run the deploy script to update the server:"
echo "./deploy.js"
echo ""
echo "The deploy will:"
echo "- Update the Caddyfile with CSP fixes"
echo "- Pull the new Docker images"
echo "- Restart services"