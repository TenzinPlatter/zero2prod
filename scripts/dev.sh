#!/usr/bin/env bash

# This script sets up the development environment for the project.
# Usage: source ./scripts/dev.sh

set -eo pipefail

# Export environment variables
export DATABASE_URL="postgresql://app:secret@localhost:5432/newsletter"
export APP_ENVIRONMENT="development"

echo "Environment variables set:"
echo "  DATABASE_URL=$DATABASE_URL"
echo "  APP_ENVIRONMENT=$APP_ENVIRONMENT"

# Start docker compose
echo "Starting Postgres container..."
docker compose up -d

# Wait for postgres to be ready
echo "Waiting for Postgres to be ready..."
until docker compose exec -T postgres pg_isready -U app -d newsletter &>/dev/null; do
  echo "Postgres is unavailable - sleeping"
  sleep 1
done

echo "Postgres is ready!"

# Run migrations
echo "Running database migrations..."
sqlx migrate run

# Start the app with cargo watch
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  echo "Starting application with cargo watch..."
  cargo watch -x run | bunyan
fi
