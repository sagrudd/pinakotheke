#!/bin/sh
# SPDX-License-Identifier: MPL-2.0
set -eu
exec python3 "$(dirname "$0")/check_upgrade_rollback.py"
