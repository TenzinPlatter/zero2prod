#!/usr/bin/env bash

DB_PORT="${POSTGRES_PORT:-5432}"
SUPERUSER="${SUPERUSER:-postgres}"
SUPERUSER_PWD="${SUPERUSER_PWD:-postgres}"
APP_USER="${APP_USER:=app}"
APP_USER_PWD="${APP_USER_PWD:=secret}"
APP_DB_NAME="${APP_DB_NAME:=newsletter}"
CONTAINER_NAME="${CONTAINER_NAME:=postgres}"
SKIP_DOCKER="${SKIP_DOCKER:=false}"
in_ci="false"
testing="false"

function setup_db_container() {
    if [[ "$SKIP_DOCKER" == "true" ]] || [[ "$(docker ps -aq -f name=${CONTAINER_NAME})" ]]; then
        echo Skipping docker setup
        return
    fi

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
    docker exec "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${CREATE_QUERY}"

    # Grant create db privileges to the app user
    GRANT_QUERY="ALTER USER ${APP_USER} CREATEDB;"
    docker exec "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${GRANT_QUERY}"
}

# ENTRYPOINT
set -eo pipefail
set -x

if [ -n "$CI" ]; then
    in_ci="true"
fi

if [ -n "$TESTING" ]; then
    testing="true"
fi

if !command -v sqlx &> /dev/null; then
    echo "sqlx could not be found, please install it"
    exit 1
fi

# if the postgres container doesn't already exist and we are in CI as if we are not in CI we will
# be connecting to supabase db and don't need to spin up a local container
if [[ "$in_ci" == "true" ]] || [[ "$testing" == "true" ]]; then
    setup_db_container
fi

>&2 echo "Postgres is up and running on port ${DB_PORT}"

if [[ "$in_ci" == "true" ]] || [[ "$testing" == "true" ]]; then
    export DATABASE_URL="postgres://${APP_USER}:${APP_USER_PWD}@localhost:${DB_PORT}/${APP_DB_NAME}"
else
    source .env
fi

sqlx database create
sqlx migrate run
