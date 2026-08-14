#!/usr/bin/env bash
set -u
root='D:/wit-bindgen'; confirmed=0; simulate_dirty=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --confirmed) confirmed=1 ;;
    --simulate-dirty) simulate_dirty=1 ;;
    --help) echo "Usage: $0 [--simulate-dirty] [--confirmed]"; exit 0 ;;
    *) echo 'BLOCKED: FORK_NOT_READY argument'; exit 43 ;;
  esac
  shift
done
if [ "$simulate_dirty" -eq 1 ]; then echo 'BLOCKED: FORK_NOT_READY dirty'; exit 43; fi
[ "$confirmed" -eq 1 ] || [ "${FORK_CONFIRMED:-0}" = 1 ] || { echo 'BLOCKED: FORK_NOT_READY unconfirmed'; exit 43; }
branch=$(git -C "$root" branch --show-current 2>/dev/null || true)
[ "$branch" = feat/assemblyscript-no-async ] || { echo 'BLOCKED: FORK_NOT_READY branch'; exit 43; }
[ -z "$(git -C "$root" status --porcelain 2>/dev/null || true)" ] || { echo 'BLOCKED: FORK_NOT_READY dirty'; exit 43; }
head=$(git -C "$root" rev-parse HEAD) || exit 43
echo 'GENERATION_COMMAND: wit-bindgen assemblyscript ./wit --out-dir bindings'
printf '%s %s\n' "$head" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > .generation-started
