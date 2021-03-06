#!/bin/bash
set -xe

if [ -z "$RUST_LOG" ]; then
    echo "Setting RUST_LOG=info for your convenience."
    export RUST_LOG=info
fi

target="networks/basic"
cd "$(git rev-parse --show-toplevel)"

# TODO: Much of the setup logic below is boilerplate.  Maybe the
# common parts of this script and setup.sh/teardown.sh should be
# extracted.
if [ -z "${CHUTNEY_PATH}" ]; then
    # CHUTNEY_PATH isn't set; try cloning or updating a local chutney.
    if [ -d chutney ]; then
	(cd ./chutney && git pull)
    else
	git clone https://gitlab.torproject.org/tpo/core/chutney
    fi
    CHUTNEY_PATH="$(pwd)/chutney"
    export CHUTNEY_PATH
else
    # CHUTNEY_PATH is set; tell the user so.
    echo "CHUTNEY_PATH is ${CHUTNEY_PATH}; using your local copy of chutney."
fi

if [ ! -e "${CHUTNEY_PATH}/${target}" ]; then
    echo "Target network description ${CHUTNEY_PATH}/${target} not found."
    exit 1
fi

"${CHUTNEY_PATH}/chutney" configure "${CHUTNEY_PATH}/$target"
"${CHUTNEY_PATH}/chutney" start "${CHUTNEY_PATH}/$target"
CHUTNEY_START_TIME=180 "${CHUTNEY_PATH}"/chutney wait_for_bootstrap "${CHUTNEY_PATH}/$target"
"${CHUTNEY_PATH}"/chutney verify "${CHUTNEY_PATH}/$target"
# TODO (end of boilerplate)

cargo run -p arti-bench --release -- -c "${CHUTNEY_PATH}/net/nodes/arti.toml" "$@"

"${CHUTNEY_PATH}"/chutney stop "${CHUTNEY_PATH}/$target"

