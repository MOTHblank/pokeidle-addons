# Hunt pacing — live server observation

This is the offline/diagnostic data-generation path for replacing the borrowed
`/hunt` pacing table with Moth measurements. It is intentionally separate from
normal Hunt Atlas forecasts.

## Architecture

```text
existing native PokéIdle WebSocket
        ↓
already-decoded campo.init / campo / batalha / estado
        ↓
opt-in JSONL trace
        ↓
strict sequence reducer
        ↓
per-map statistics + coverage
        ↓
Moth pacing JSON / compatible TSV
```

There is no second WebSocket, no polling and no map-path simulator. Map JSON and
collision geometry are not inputs to the reducer.

## Collection

The runtime hook is `src/hunt_pacing_trace.rs`. It is off unless
`MOTH_HUNT_PACING_TRACE=1` or `MOTH_HUNT_PACING_TRACE_DIR` is set.

Use `hunt-pacing collect` to print the current environment switches. For controlled
runs, also set `MOTH_HUNT_PACING_RUN_ID` and
`MOTH_HUNT_PACING_EXPECT_HUNT`. The run ID is carried into reduced windows;
when an expected hunt is set, packets from any other hunt are never admitted to
that run's pacing aggregate. Game
version is taken from the authenticated welcome when exposed by the server;
`MOTH_HUNT_PACING_GAME_VERSION` can override it for controlled collection.
The default trace directory is:

```text
<Moth data root>/diagnostics/hunt-pacing/raw/
```

Each JSONL packet record keeps both receipt time and the server timestamp when
present. Kill windows use the server clock when its elapsed interval is
consistent with the monotonic receipt interval; otherwise the reducer falls
back to the local monotonic clock instead of mixing clock domains. For `campo.init`, `mapa ?? slug` is the authoritative map identity;
the current hunt slug is recorded separately. Field records retain the server's
actual movement contract (`dc/dr -> c/r`, `em`, `ms`) for the hero and every
mob, plus spawn/death/removal/respawn and target state. Battle records retain
the full event list and normalized wild-death boundaries.

The calibration snapshot contains the active Pokémon identity/species/level,
IVs, server battle stats, held item, TM/AOE state, bicycle rarity/factor,
seasonal type, Outland tier, gym state, automation state and relevant bonuses.
The raw `estado` packet is also retained, so fields not yet normalized can be
used by a later reducer without recollection.

### Calibration profiles

Set `MOTH_HUNT_PACING_PROFILE` deliberately:

- `st1`: on foot, single target, configured so every wild dies in exactly one
  damaging attack.
- `aoe`: on foot, AOE enabled, stable one-shot setup. The trace retains the
  actual number of deaths in each server battle batch; no two-target assumption
  is made.
- `bike-validation`: collect a controlled map with each bicycle rarity. These
  samples are retained separately and never become baseline `s`.
- `deadtime-validation`: deliberately collect 1/2/3/multi-hit setups for
  fitting the fixed/walking dead-time model. Set
  `MOTH_HUNT_PACING_HITS` and `MOTH_HUNT_PACING_COMBAT_SECONDS` for the
  controlled setup so the report can compare observed total pacing against the
  exact forecast curve rather than guessing combat duration from packet shape.
- `diagnostic`: raw capture only. It is never admitted to pacing aggregates.

The profile label is a calibration assertion, not a substitute for validation:
the reducer rejects observable contradictions (bike/AOE mismatch, multiple
explicit attack actions, multi-kill ST1 batches, active Pokémon death, healing,
special combat, reconnects, state fingerprint changes and large packet gaps).
Some server death receipts do not repeat the full moveset or every damage
detail, which is why the raw battle traffic is retained. As the protocol fields
are confirmed, stricter one-hit checks can be added to the reducer and applied
to old traces.

## Reduction

`reduce` operates independently on each trace file. A sequence is split on:

- connection generation;
- scene/map generation;
- authoritative map ID;
- calibration fingerprint (active Pokémon/combat state, bike, AOE/TM, seasonal,
  Outland, gym and automation state);
- a configurable significant packet gap.

Warm-up discards at least `--warmup-kills` and always ends on a complete server
kill batch. For ST1 every batch is one kill. For AOE this prevents a measurement
from starting halfway through a multi-kill attack. The first post-warm-up batch
is the sequence boundary; kills in subsequent batches are measured:

```text
seconds_per_kill =
    (last batch boundary - first batch boundary)
    / kills in subsequent batches
```

This includes real walking, target changes, replenishment, enemy approach and
server pacing. It never measures spawn radii. Each reduced window also retains
the measured kill boundaries, server and receipt kill timestamps, the
previous-kill timestamp, the first subsequent target selection and the first
subsequent **real position transition** when those fields are observed. A
stationary `campo` entity update is not counted as movement.

Default outlier handling is deliberately simple and auditable: impossible
windows outside 0.20–120 seconds/kill are rejected; with at least five
independent windows for one map/profile, values more than 6 MAD from the median
are excluded. All thresholds are command-line options. The reducer preserves
window samples, kills, total seconds, weighted mean, median, p10, p90 and MAD so
the production aggregate can be changed later without recollecting.

## Output and coverage

`all <raw> <out>` writes:

```text
reduced.json
moth-pacing.json
moth-pacing.tsv
coverage.txt
next-plan.json
next-plan.txt
```

The JSON is the rich source of truth. The reducer refuses to combine traces
that report multiple known game versions; cross-version pacing must be reduced
separately. The TSV uses the same logical records as the existing
`ritmo-v1409.tsv` loader. It only emits a map row once both ST1
and AOE have been measured; missing data is never fabricated.

Coverage uses the exact 500 map IDs (and 1,043 hunt identifiers) in the current
borrowed table only as the target corpus. Map measurements and hunt→map
identities in the Moth output come from live `campo.init` observations. The
coverage report prints Moth-vs-`/hunt` deltas for measured maps and lists
observed map IDs shared by multiple hunt slugs, so the initial reproducibility
check is visible directly in the generated report.
"Enough" currently means at least three independent valid windows; each window
already has the configured minimum measured kills.

The first validation milestone is 5–10 deliberately different maps: open,
corridor-heavy, cave/multi-level, Outland, shared-map hunts and a generated map.
Repeated independent ST1 windows should converge, and hunts that report the
same authoritative map ID should reduce to the same per-map pacing value.


## Validation reports

The JSON export and coverage report include two model checks that use the same
shared pacing-math module as normal Hunt Atlas forecasts. `validate <reduced.json>`
prints only these checks when iterating on controlled calibration runs.

For `bike-validation`, the measured ST1 value on that map is the on-foot
baseline. Every collected bicycle factor is compared with:

```text
walk0 = max(0, 0.83 * sMap - 1.41)
fixed = sMap - walk0
predicted = fixed + walk0 / bicycle_factor
```

For `deadtime-validation`, the controlled combat duration is compared with:

```text
remainingWalk = walk0 * exp(-fightSeconds / 2.8)
predicted = fightSeconds + fixed + remainingWalk
```

The report prints observed seconds/kill, predicted seconds/kill and relative
error. Validation traces stay outside the production `s`/`sAoe` aggregates.


## Calibration planner

`hunt-pacing plan [reduced.json] [--count N] [--json plan.json]` turns
coverage into a concrete list of controlled collection jobs.

With no reduced dataset, it produces a deliberately varied pilot queue:
shared-map convergence, generated-map, Outland-style and ordinary unique-map
work. With a reduced dataset it first finishes already observed live maps, then
adds a second hunt where useful to test shared-map convergence, and finally
fills the queue with diverse new pilot work.

When traces contain a current authenticated account hunt catalog, the planner
filters candidates to hunts present in at least one collected account and lists
which controller profile(s) can run each job. Older reductions without that
metadata remain usable and simply show account availability as unknown.

Each job contains:

- a stable run ID;
- the hunt slug that this run is allowed to measure;
- account profile(s) whose authoritative server catalog contains that hunt;
- ST1 or AoE profile;
- target/completed independent windows;
- a borrowed map ID **hint** for operator orientation only;
- the exact `MOTH_HUNT_PACING_*` environment values to set.

The borrowed map hint is never used by the reducer as ground truth. The
authoritative map identity still comes only from live `campo.init.mapa ?? slug`.

`hunt-pacing all` writes both `next-plan.json` and `next-plan.txt` after
every reduction, so collection can resume from the current coverage state
without maintaining a separate checklist.
