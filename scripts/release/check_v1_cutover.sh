#!/bin/sh
# SPDX-License-Identifier: MPL-2.0
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
exec python3 "$repository_root/scripts/release/check_v1_cutover.py" "$@"
