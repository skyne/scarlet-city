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

Open the printed URL. **WASD** is camera-relative on foot (**W** walks up the isometric screen). Mouse aims (facing wedge + reticle). Click shoot, **E** enter/exit vehicles or pick loot, **Space/C** climb/descend in the hovercraft, **Shift** brake, **F3** debug overlay (fps, ping, tick). Stand in an extraction pad (docks or tower helipad) to fill the extract meter.

## Architecture

- The server owns physics, altitude, vehicle drift, LOS, loot, and extraction.
- Overlap in `(x, y)` only blocks when height intervals overlap, so a hovercraft can pass over a low shed and still slam into a tower.
- The client projects `(x, y, z)` with `screenX = (x - y) * scale`, `screenY = (x + y) * 0.5 * scale - z * zScale`, and draws a ground drop-shadow when `z > 0.01`.
- Networking and session state live in C# (`IGameTransport`, `GameSession`). Canvas and keyboard/mouse are behind `IRenderer` / `IInputSource` for a later Tauri or Electron wrap.
