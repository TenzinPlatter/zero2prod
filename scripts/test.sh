export CONTAINER_NAME="postgres_test"
export POSTGRES_PORT="12345"
export DATABASE_URL="postgresql://app:secret@localhost:12345/newsletter"

if [[ ! -f ./scripts/init_db.sh ]]; then
    echo "Please run the testing script from the project root"
    exit 1
fi

set -eo pipefail
set -x

if [[ -z "${CI}" ]]; then
    export CONFIG_FILE="configuration.local.yaml"
else
    # overload port and url in CI as we don't need to worry about port being in use
    export POSTGRES_PORT="5432"
    export DATABASE_URL="postgresql://app:secret@localhost:5432/newsletter"
    export CONFIG_FILE="configuration.ci.yaml"
fi

if [[ $(docker ps -aq -f name=${CONTAINER_NAME}) ]]; then
    docker kill ${CONTAINER_NAME} || true
    docker rm ${CONTAINER_NAME}
fi

TESTING=true ./scripts/init_db.sh
if [[ -z "${CI}" ]]; then
    TEST_LOG=1 cargo nextest run
else
    # don't use nextest in CI to avoid installing it every time
    TEST_LOG=1 cargo test run
fi

