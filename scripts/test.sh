export CONTAINER_NAME="postgres_test"
export POSTGRES_PORT="12345"
export DATABASE_URL = "postgresql://app:secret@localhost:12345/newsletter"

if [[ ! -f ./scripts/init_db.sh ]]; then
    echo "Please run the testing script from the project root"
    exit 1
fi

set -eo pipefail
set -x

if [[ -z "${CI}" ]]; then
    export CONFIG_FILE="configuration.local.yaml"
else
    export CONFIG_FILE="configuration.ci.yaml"
fi

if [[ $(docker ps -aq -f name=${CONTAINER_NAME}) ]]; then
    docker kill ${CONTAINER_NAME} || true
    docker rm ${CONTAINER_NAME}
fi

TESTING=true ./scripts/init_db.sh
TEST_LOG=1 cargo nextest run
