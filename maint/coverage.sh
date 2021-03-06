#!/bin/bash

function usage()
{
    cat <<EOF
${SCRIPT_NAME}: Generate coverage using grcov.

Usage:
  coverage.sh [opts] [suites...] : Run the provided test suites.

Options:
  -h: Print this message.
  -c: Continue using data from previous runs. (By default, data is deleted.)

Suites:
  "unit": equivalent to cargo test --all-features
  "integration": a simple integration test with a chutney network.
  "all": enables all suites.

Notes:
  You need to have grcov, rust-nightly, and llvm-tools-preview installed.
  For integration tests, you'll need chutney and tor.
EOF
}

set -e

TOPDIR=$(dirname "$0")/..
cd "$TOPDIR"

CLEAR=yes
UNIT=no
INTEGRATION=no

while getopts "ch" opt ; do
    case "$opt" in
	c) CLEAR=no
	   ;;
	h) usage
	   exit 0
	   ;;
	*) echo "Unknown option. (Run '$0 -h' for help.)"
	   exit 1
	   ;;
    esac
done

# Remove the parsed flags.
shift $((OPTIND-1))

for suite in "$@"; do
    case "$suite" in
	unit) UNIT=yes
	      ;;
	integration) INTEGRATION=yes
		     ;;
	all) UNIT=yes
	     INTEGRATION=yes
	     ;;
	*) echo "Unrecognized test suite '$suite'. (Run '$0 -h' for help.)"
	   exit 1
	   ;;
    esac
done

if [ "$UNIT" = no ] && [ "$INTEGRATION" = no ]; then
    echo "No test suites listed; nothing will be done. (Run '$0 -h' for help.)"
    exit 1
fi

if [ "$CLEAR" = yes ] ; then
    # Clear the old coverage report.  We do this by default unless
    # we are given the -c option.
    ./maint/with_coverage.sh -s /bin/true
fi

if [ "$UNIT" = yes ] ; then
    # Run the unit tests, with coverage.
    ./maint/with_coverage.sh -c -s cargo test --all-features
fi

if [ "$INTEGRATION" = yes ] ; then
    # Run the integration tests, with coverage.
    #
    # (This is just a basic test that uses curl over Arti over a
    # chutney network. It's taken from the gitlab-ci tests.)

    # TODO: we might want, at some point, to have a the following stuff
    # go into a basic extensible integration-testing script that gets
    # run both from here and from the .gitlab-ci.yml file.
    trap ./tests/chutney/teardown.sh 0
    ./maint/with_coverage.sh -c -s ./tests/chutney/setup.sh
    curl http://example.com -vs --socks5-hostname 127.0.0.1:9150 -o /dev/null
    trap - 0
    ./tests/chutney/teardown.sh
fi

# Generate the coverage report.
./maint/with_coverage.sh -c /bin/true

