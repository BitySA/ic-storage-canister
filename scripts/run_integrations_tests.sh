#!/bin/bash

ulimit -n 65536

./scripts/build.sh

cargo test -p integration_tests -- --test-threads=1