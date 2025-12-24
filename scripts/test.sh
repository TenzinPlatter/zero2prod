export CONTAINER_NAME="postgres_test"
export POSTGRES_PORT="12345"
export DATABASE_URL="postgresql://app:secret@localhost:12345/newsletter"

if [[ ! -f ./scripts/init_db.sh ]]; then
    echo "Please run the testing script from the project root"
    exit 1
fi

set -eo pipefail
set -x

if [[ -n "${CI}" ]]; then
    # overload port and url in CI as we don't need to worry about port being in use
    export POSTGRES_PORT="5432"
    export DATABASE_URL="postgresql://app:secret@localhost:5432/newsletter"
    export CONFIG_FILE="configuration.ci.yaml"
    export APP_ENVIRONMENT="CI"
else
    export APP_ENVIRONMENT="LOCAL"
fi

TESTING=true ./scripts/init_db.sh
if [[ -z "${CI}" ]]; then
    TEST_LOG=debug cargo nextest run --no-capture --no-fail-fast "$@"
else
    # don't use nextest in CI to avoid installing it every time
    TEST_LOG=debug cargo test "$@" -- --nocapture
fi

