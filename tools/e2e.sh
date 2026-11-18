#!/usr/bin/env bash
# In-game check for Eternal Starlight slice 1: the Starlight dimension exists,
# is enterable and generates four of its biomes.
#
# THIS RUN HAS NEVER BEEN MADE: no launcher exists at the SDK pin, so the mod
# cannot be installed into a game here. `notes.md` records the in-game half as a
# gap; this script is what the next slice runs.
#
# The mod is a flat `.hmod` the launcher compiles into the game, so GAME must
# name a game the launcher built with it:
#   horizon-launcher native-build ports/eternal_starlight --instance <instance> --trust-native-code
# and then `GAME=<that horizon-client> tools/e2e.sh`. CONTROL_GAME is an
# unmodified game of the same build; without it the control leg is skipped and
# the run says so.
#
# Leg A (the modded game):
#   1. `/execute in eternal_starlight:starlight run …` answers: the dimension is
#      registered (it is not a vanilla level);
#   2. the player teleported into the level reads `state.dimension` as
#      `eternal_starlight:starlight`;
#   3. a column of the generated terrain holds the mod's own blocks — grimstone
#      (the noise settings' default block) and one of the surface blocks — which
#      an unmodified game cannot place, because it has no such blocks;
#   4. `/eternal_starlight status` (a native command, so it is sent through chat)
#      names the dimension, `4 biomes`, and a biome id from the slice, at two
#      distant positions.
# Leg B (the unmodified game): step 1 is refused and the `status` command is
#   unknown, so every Leg A answer is the mod's and not the engine's.
#
# SELFTEST=1 reruns Leg B's first assertion inverted — the control game must
# accept the dimension — which must fail: that proves the assertion is not
# vacuous. The failure is the expected result, and the run says so.
set -euo pipefail
EXIT_STATUS=1
cleanup() {
    local status=$?
    [ -n "${SOCK:-}" ] && rm -f "$SOCK"
    echo "EXIT:$status"
    exit "$status"
}
trap cleanup EXIT
trap 'echo "FAILED at line $LINENO: $BASH_COMMAND" >&2' ERR

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/eternal_starlight
SHARED=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$SHARED/horizon-ctl}
GAME=${GAME:-}
CONTROL_GAME=${CONTROL_GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/eternal-starlight-e2e.sock}
SELFTEST=${SELFTEST:-0}
DIM=eternal_starlight:starlight

[ -x "$CTL" ] || { echo "missing binary: $CTL" >&2; exit 1; }
[ -n "$GAME" ] || {
    echo "missing binary: GAME is unset. The mod is a launcher-built .hmod: name the" >&2
    echo "horizon-client the launcher produced with it (notes.md, 'In-game')." >&2
    exit 1
}
[ -x "$GAME" ] || { echo "missing binary: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}"/eternal-starlight-home.XXXXXX)
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"
rm -f "$SOCK"
cd "$SCRATCH"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { echo "== $* =="; }
fail() { echo "FAIL: $*" >&2; exit 1; }

launch() { # game world
    ctl launch --game "$1" --world "$2" --seed 20261105 --creative --headless \
        --wait --resources "$RES"
}

# The console's answer to `line`, as one line of JSON.
console() { ctl command "$1"; }

# The player's chat answer: native mod commands hear chat, not the console path.
chat() {
    ctl chat "$1"
    ctl wait 2 >/dev/null
    ctl messages 20 | jq -r '.chat[]' 2>/dev/null
}

dimension_of() { ctl state | jq -r '.player.dimension'; }

# ----------------------------------------------------------------- Leg A
say "modded game: $GAME"
launch "$GAME" starlight-e2e
dimension_ok=0
if console "execute in $DIM run setblock 0 70 0 minecraft:stone" \
    | jq -e '.ok == true' >/dev/null 2>&1; then
    dimension_ok=1
fi

console "execute in $DIM run tp @s 0 120 0" >/dev/null
ctl wait 5 >/dev/null
dimension=$(dimension_of)
[ "$dimension" = "$DIM" ] || fail "player is in $dimension, not $DIM"

say "column at 0,x,0"
found_default=0
found_surface=0
for y in $(seq 40 2 110); do
    block=$(ctl block 0 "$y" 0 2>/dev/null | jq -r '.block // empty' 2>/dev/null || true)
    case "$block" in
        eternal_starlight:grimstone) found_default=1 ;;
        eternal_starlight:nightfall_dirt | eternal_starlight:nightfall_grass_block | \
            eternal_starlight:nightfall_mud | eternal_starlight:voidstone) found_surface=1 ;;
    esac
done
[ "$found_default" = 1 ] || fail "no grimstone in column 0,40..110,0: the dimension did not generate"
[ "$found_surface" = 1 ] || fail "no surface block in column 0,40..110,0: the material rule did not run"

say "status readout"
status_a=$(chat "/eternal_starlight status 0 70 0")
echo "$status_a"
echo "$status_a" | grep -q "$DIM: 4 biomes" || fail "status did not name the dimension and its 4 biomes"
echo "$status_a" | grep -qE 'is eternal_starlight:(starlight_forest|starlight_dense_forest|glimmer_scrubland|dark_swamp)' \
    || fail "status answered a biome outside the slice"
status_b=$(chat "/eternal_starlight status 4000 70 -4000")
echo "$status_b"
echo "$status_b" | grep -q "$DIM: 4 biomes" || fail "status failed at a distant position"
[ "$dimension_ok" = 1 ] || fail "the dimension refused a console command"

ctl quit >/dev/null 2>&1 || true

# ----------------------------------------------------------------- Leg B
if [ -n "$CONTROL_GAME" ]; then
    say "control game: $CONTROL_GAME"
    launch "$CONTROL_GAME" starlight-control
    control_ok=0
    if console "execute in $DIM run setblock 0 70 0 minecraft:stone" \
        | jq -e '.ok == true' >/dev/null 2>&1; then
        control_ok=1
    fi
    unknown=$(chat "/eternal_starlight status 0 70 0")
    echo "$unknown"
    if [ "$SELFTEST" = 1 ]; then
        ctl quit >/dev/null 2>&1 || true
        [ "$control_ok" = 1 ] || fail "SELFTEST: the control game refused the dimension, so this assertion proves nothing here"
        fail "SELFTEST: the control game accepted the dimension; that must not happen"
    fi
    [ "$control_ok" = 0 ] || fail "the unmodified game accepted $DIM: the level is not the mod's"
    echo "$unknown" | grep -q "4 biomes" && fail "the unmodified game answered the mod's status command"
    ctl quit >/dev/null 2>&1 || true
else
    say "no CONTROL_GAME: the control leg is skipped (not evidence on its own)"
fi

echo "PASS: the Starlight dimension is registered, enterable and generates the slice's blocks"
EXIT_STATUS=0
