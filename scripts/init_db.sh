#!/usr/bin/env bash
set -eo pipefail
set -x

if !command -v sqlx &> /dev/null; then
    echo "sqlx could not be found, please install it"
    exit 1
fi

# Check if a custom parameter has been set, if not, set a default value

DB_PORT="${POSTGRES_PORT:-5432}"
SUPERUSER="${SUPERUSER:-postgres}"
SUPERUSER_PWD="${SUPERUSER_PWD:-postgres}"
APP_USER="${APP_USER:=app}"
APP_USER_PWD="${APP_USER_PWD:=secret}"
APP_DB_NAME="${APP_DB_NAME:=newsletter}"

CONTAINER_NAME="postgres"

# Remove any existing container with the same name
if [ ! "$(docker ps -aq -f name=${CONTAINER_NAME})" ]; then
    docker run \
        --env POSTGRES_USER="${SUPERUSER}" \
        --env POSTGRES_PASSWORD="${SUPERUSER_PWD}" \
        --publish "${DB_PORT}":5432 \
        --name "${CONTAINER_NAME}" \
        --detach \
        postgres -N 1000

    until docker exec $CONTAINER_NAME pg_isready; do
        sleep 1
    done
    
    # Create the application user
    CREATE_QUERY="CREATE USER ${APP_USER} WITH PASSWORD '${APP_USER_PWD}';"
    docker exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${CREATE_QUERY}"

    # Grant create db privileges to the app user
    GRANT_QUERY="ALTER USER ${APP_USER} CREATEDB;"
    docker exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${GRANT_QUERY}"

fi

>&2 echo "Postgres is up and running on port ${DB_PORT}!"

source .env
sqlx database create
sqlx migrate run
