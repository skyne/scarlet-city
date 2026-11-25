# Scarlet City

Monorepo skeleton for a multiplayer top-down/isometric extraction shooter. World space is `(x, y, z)` from day one so rooftops, bridges, and flying vehicles share the same collision and render path.

| Path | Role |
| --- | --- |
| `/shared/protocol/scarlet.spdl` | Protocol source of truth (SPDL) |
| `/shared/codegen` | Generates Rust + C# from SPDL |
| `/shared/maps/map_01.json` | Starter city block |
| `/server` | Authoritative Rust simulation + WebSocket |
| `/client` | Blazor WebAssembly + Canvas 2D isometric view |

## Protocol language (SPDL)

Edit `shared/protocol/scarlet.spdl`, then:

```bash
python3 shared/codegen/generate.py
```

That rewrites `server/src/protocol.rs` and `client/Protocol/*.cs`. Unions are internally tagged JSON (`"type": "Join"`).

## Assets

Binary game assets are not stored in git. Fetch them before running the client:

```bash
./scripts/fetch-assets.sh
```

This downloads `scarlet-city-assets.tar.gz` from the `assets-v1` GitHub release via `gh` and extracts into `client/wwwroot/assets/`.

## Run

Terminal 1 — server (listens on `ws://localhost:3030/ws`):

```bash
cd server
cargo run
```

Terminal 2 — web client (`http://localhost:5173`):

```bash
export PATH="$HOME/.dotnet:$PATH"   # if the SDK was installed via dotnet-install
cd client
dotnet run
```

Open the printed URL. **WASD** is camera-relative on foot (**W** walks up the isometric screen) and stays held while the mouse aims. Chromium sometimes emits a `keyup` for a key you are still holding as the pointer moves; the client keeps that key down until the pointer settles, and key-repeat re-asserts it, so move and aim share one input sample.

In the hideout, pick **1–4** crew (solo, duo, trio, or a full squad). **1–4** or **Tab** swaps who you control. Click an **[AI]** card to order that crewmate, or leave none selected to order every AI teammate. **G** cycles Hold / Follow / Engage at the cursor. **Middle click** or **Alt+click** sends Move / Drive / Fly to that point. Click shoot, **E** enter/exit vehicles or pick loot, **Space/C** climb/descend in the hovercraft, **Shift** brake, **F3** debug overlay (fps, ping, tick). Stand in an extraction pad (docks or tower helipad) to fill the extract meter.

An optional squad id on the deploy screen joins a co-op squad that still has room (four actors total). Each human keeps a distinct body. Swapping only works onto a living crewmate nobody else is controlling. The isometric camera eases over to the new body in about a quarter of a second.

Deploy without a squad id and you share the city with every other crew. You can fight them for a heist bag, or leave them alone.

## Heists, heat, and rival crews

`map_01` has three jobs: **South Block Vault** (rooftop hack or street breach), **East Tower Terminal** (roof terminal or street breach), and **North Warehouse Cargo** (side door or street breach).

- Hold **E** on the blue ring for the quiet approach. Rooftop points need a hovercraft parked slow over the marker. That opens the site without an immediate alarm, then a silent alarm follows.
- Hold **E** on the red ring for a fast loud breach. That trips the alarm immediately.
- The finished job drops a **heist bag**. Pick it up and stand on an extraction pad. The bag is heavy loot; dying drops it for whoever is closest.
- Sites move through Ready, InProgress, Breached, Looted, and Cooldown. After the cooldown the job can be run again.

Heat is per squad, from `0.0` to `5.0` (stars `0`–`5`). Unsuppressed gunfire, loud breaches, vehicle theft, and shooting guards or police push it up. It falls slowly only after the fight ends and no hostile unit can see the crew. Guards patrol the jobs. At two stars and above the server dispatches police cars, then SWAT, toward the last noise or alarm — not through tall cover they cannot see past.

A heist alarm is broadcast as an event and a site marker. Nearby crews hear that the building is hot. Player bodies and gunfire pings still require line of sight, so a tower blocks a wall-hack even when the alarm is audible. The HUD shows heat, each objective, and hot-zone lines. The isometric view marks jobs, alarm rings, hostile units, and police vehicles, with a ground shadow when they are off the street.

## Architecture

- The server owns physics, altitude, vehicle drift, LOS, loot, extraction, heist state, heat, hostile patrols and police response, crew roster, character swap, and AI orders (Follow, Hold, Move/Drive/Fly, Engage) at 20 Hz.
- Each profile stores up to four specialists in `game.db` (Point, Driver, Scout, Heavy). Deploy spawns the ones you selected; the rest of a squad is AI until a co-op player takes a body.
- Overlap in `(x, y)` only blocks when height intervals overlap, so a hovercraft can pass over a low shed and still slam into a tower.
- The client projects `(x, y, z)` with `screenX = (x - y) * scale`, `screenY = (x + y) * 0.5 * scale - z * zScale`, and draws a ground drop-shadow when `z > 0.01`.
- Networking and session state live in C# (`IGameTransport`, `GameSession`). Canvas and keyboard/mouse are behind `IRenderer` / `IInputSource` for a later Tauri or Electron wrap.
