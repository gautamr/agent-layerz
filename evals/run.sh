#!/usr/bin/env bash
# Usage: evals/run.sh <eval-dir> <layer-dir>
# Builds the eval's fixture repository with <layer-dir> stacked first, runs the task headless and
# prints one JSON result line. Run only inside a throwaway container.
set -uo pipefail
eval_dir="$(cd "${1:?eval dir}" && pwd)"
layer="$(cd "${2:?layer dir}" && pwd)"
read -r url commit < "$eval_dir/repo"
turns="$(cat "$eval_dir/budget" 2>/dev/null || echo 40)"
work="$(mktemp -d)"
git clone -q "$url" "$work/repo" && git -C "$work/repo" checkout -q "$commit"
cat > "$work/repo/layer.yaml" <<YAML
stack:
  - source: path:$layer
  - source: path:./.layer
targets: [claude]
YAML
mkdir -p "$work/repo/.layer" && printf 'name: eval\nkind: repo\nvisibility: private\nversion: 0.0.0\n' > "$work/repo/.layer/layer.yaml"
(cd "$work/repo" && alz build --adopt >/dev/null) || { echo "{\"eval\":\"$(basename "$eval_dir")\",\"error\":\"alz build failed\"}"; exit 1; }
start=$(date +%s)
out=$(cd "$work/repo" && claude -p "$(cat "$eval_dir/task.md")" --output-format json --max-turns "$turns" --dangerously-skip-permissions 2>/dev/null)
secs=$(( $(date +%s) - start ))
(cd "$work/repo" && bash "$eval_dir/check.sh" >/dev/null 2>&1) && pass=true || pass=false
cost=$(echo "$out" | jq -r '.total_cost_usd // "unknown"' 2>/dev/null)
version=$(grep -m1 '^version:' "$layer/layer.yaml" | awk '{print $2}')
echo "{\"eval\":\"$(basename "$eval_dir")\",\"layer\":\"$(basename "$layer")@${version:-?}\",\"pass\":$pass,\"seconds\":$secs,\"cost_usd\":\"$cost\"}"
rm -rf "$work"
