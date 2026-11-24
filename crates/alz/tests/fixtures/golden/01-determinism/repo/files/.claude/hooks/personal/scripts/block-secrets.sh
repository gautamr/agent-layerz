#!/bin/sh
# Reached only when the hook's `if` rule matched a secret path.
echo "Blocked by agent-layerz: secret files are off limits" >&2
exit 2
