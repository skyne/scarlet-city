# Customizable 3D characters for Scarlet City

Research note, checked against live project pages on **25 November 2026**. This file records what those pages say. It is not a license opinion, and it does not clear any asset for shipping. Save the license file that ships inside each download and re-read it before a character mesh, texture, or baked sprite goes into the game.

Nothing in this pass was imported into Blender, and no sample file was downloaded. Polygon counts below are the counts the upstream docs state for a named mesh. Other filenames that look numeric still need a mesh inspection.

## What the game can use today

Scarlet City is a multiplayer heist/extraction game. The server simulates `(x, y, z)`. The client is .NET Blazor WebAssembly. Drawing goes through `IRenderer` into a 2D canvas (`client/wwwroot/js/isoRenderer.js`): filled polygons, ellipses, lines, text. There is no glTF loader and no skinned-mesh path.

The projection in `IsoProjection` is 2:1 dimetric, with altitude:

- `screenX = (x - y) * scale`
- `screenY = (x + y) * 0.5 * scale - z * zScale`

`PlayerState.yaw` is a continuous float (mouse aim). A sprite sheet has to quantize that yaw. A later WebGL renderer behind `IRenderer` could follow yaw directly. Packaging with Tauri or Electron does not add that renderer by itself.

So a customizable human is an **offline art pipeline** until the client grows a 3D backend. Body sliders, hair, and clothing are authored in Blender, then either baked into the canvas or, later, exported as glTF.

## Recommendation

Start with **archetype bodies plus a few rigid slots**, and **bake 8-direction isometric sprite sheets** for the canvas that exists now. Keep the Blender sources so the same characters can become glTF if the client grows a WebGL renderer.

Practical first set:

1. Author characters in **MPFB** (MakeHuman for Blender). Use the CC0 core basemesh and targets. Leave the MakeHuman application (AGPL) and the MPFB add-on (GPL) in the art toolchain. Ship the exported mesh or the baked sprites.
2. Build a handful of frozen archetypes that share one topology and one skeleton: for example a narrower and a broader build, in the gender presentations the roster needs. Continuous sliders stay in the Blender file.
3. Fit jackets and pants **on each archetype**, run MPFB’s refit, then freeze that pair. Treat hats, masks, glasses, and backpacks as the first real swap slots. Those pieces move less when the torso changes.
4. Bake each shipping loadout to an 8-facing sheet from a camera that matches `IsoProjection`, including a ground-contact frame so the existing drop shadow still reads at `z > 0`. Sixteen facings are the follow-up if 45° steps look wrong in motion.
5. Keep Anny available as a scripted way to generate the same MakeHuman-derived shapes (Apache-2.0 code, CC0 MakeHuman data). Stay on Anny’s own topology. The SMPL-X topology Anny can download is marked non-commercial on the Anny README.
6. Use Quaternius or Kenney only as placeholder or stylized kits after reading that kit’s own license file. They do not replace a body-slider system.
7. Leave MB-Lab / CharMorph base meshes out of a closed-source 3D export. Their license text puts the mesh database, and meshes generated from it, under AGPL. See the table.

Runtime assembly of modular glTF parts (Option A below) is the right second step once many players, smooth yaw, and live outfit swaps are worth a WebGL renderer. It is a poor first step on the current canvas.

## Comparison

| Source | Body, gender, proportions | Hair and clothes | Format and rig | License caveats to verify before shipping |
| --- | --- | --- | --- | --- |
| **MakeHuman** standalone, and **MPFB** (Blender add-on, releases through 2.0.18 on the static site) | One shared basemesh (“hm08”). Sliders move vertices and keep the same face count. Docs describe age, sex presentation, and build on that mesh. | System pack lists hair, brows, lashes, eyes, and some clothes. Community clothes are separate downloads. MPFB can refit assets after a slider change. | Standalone export docs list DAE, FBX, OBJ, STL, plus BVH. DAE default in that doc: Y-up, face +Z, **decimeter** scale. MPFB adds Rigify and a Mixamo-oriented rig variation (see the 2.0.11 / Mixamo tutorial notes). Confirm bone names on the export you actually save. | **Code:** MakeHuman AGPL, MPFB GPL (`about/license.html`, repo `LICENSE.md`). **Core assets** (basemesh, targets, skins): CC0. Project FAQ says an exported model built from those core assets may go into a closed-source commercial game, with no attribution required for the CC0 part. **Any third-party piece keeps its own license.** Asset-pack FAQ names CC0 and CC-BY as the two licenses used on those packs. |
| **Anny** (NAVER, `github.com/naver/anny`, README current through v0.6.2 notes) | MakeHuman-derived parametric body, infants through elders. Phenotype sliders and local shape changes. | Face-unit blendshapes from a CC0 face-units pack. It is a body model, not a clothing library. | Default `anny` rig: **104 bones**. `rig="makehuman"`: **163 bones**. Default topology is a triangulated MakeHuman-derived mesh; quads and the unedited MakeHuman quad mesh are separate topology names. PyTorch tool, not a Blazor runtime. | **Code:** Apache-2.0. **`data/mpfb2` and face units:** CC0, per the README. **`soma` topology:** Apache-2.0, per the README. **`smplx` topology:** README says non-commercial only, and points at `LICENSE.txt` inside a NAVER zip. README also warns the “free” install may download non-commercial assets. Use the CC0 MakeHuman topology for anything that might ship. |
| **MakeHuman system assets and community asset packs** | Same hm08 body. Proxy meshes are alternate topologies of that body. Docs define **`male1591` as 1591 faces**. The system-pack index also lists names such as `female1605`; treat the number as a hint until the mesh is counted. | System-pack page labels listed hair (`afro01`, `bob01`, `long01`, …), eyes, brows, and clothes such as `fedora01` and `female_casualsuit01` as **CC0**. Pack FAQ warns a garment made on a short, thin body can leave gaps on a taller or heavier body when delete-groups are missing. | Clothes are posed to the MakeHuman body and helpers (joint cubes, skirt helper, and similar). They are not a generic humanoid kit. Bone mismatch with a Mixamo or Quaternius rig is expected until weights are transferred. | Read the license column on the specific pack page. CC-BY needs the attribution the author asked for. The browsable asset repository and forums were taken offline in October 2026 (PHP shutdown; static site news 2026-10-21 and 2026-11-08). The static site says pack zips and `assets.json` were still online then. Confirm the file you download matches the pack page. |
| **Quaternius** base characters and modular outfits | Preset stylized bodies. The public catalog tags “Universal Base Characters” and “Modular Character Outfits - Fantasy” as rigged and retargetable. No body-slider system is described on the pages read here. | Separate outfit kits aimed at that humanoid rig. Whether a jacket is a skinned mesh, a rigid attach, or a texture swap has to be checked on the kit page. Art direction is stylized, which may fight a grounded heist read. | Humanoid, retargetable, per the catalog tags. File formats are per kit. | **Not uniformly CC0.** `quaternius.com/license.html` is the Quaternius Asset License (QAL) v1.0, last updated 8/28/2026: commercial use is allowed, credit is not required, and the assets may not be resold or redistributed as an asset pack. The FAQ says **each kit is either CC0 or QAL**, and the kit page plus the `.txt` in the zip decide which. |
| **Kenney** asset pages | Low-poly preset characters. No parametric human on the support page. | Occasional separate accessories inside a pack. Confirm on the pack page. Useful as greybox stand-ins. | Engine-agnostic downloads. Rig depth varies by pack. | `kenney.nl/support` says game assets **on the asset pages** are CC0, including commercial use, and that attribution is not required. Still open the license file in the zip. That sentence is about those asset pages, not about every paid storefront bundle. |
| **OpenGameArt.org** | Whatever the submission is. Some humans, many props. Rarely a slider-based body. | Per submission. Modular sets exist and are easy to mis-license. | Often OBJ, Blend, or FBX. Rig quality is inconsistent. | FAQ says submissions may be used in commercial projects **if that submission’s license is followed**. Credit text, when the artist wrote one, sits in the “Copyright/Attribution Notice” on the submission. Share-alike and GPL submissions are a bad fit for a closed client; skip them unless the whole shipping work can meet those terms. Dual-license wording must be read on the submission itself. |
| **Sketchfab and itch.io** | Filters and tags can surface humans and clothing. They are catalogs, not a character system. | Same. “Modular” in a title is not evidence the piece matches hm08 or any other skeleton. | glTF is common on Sketchfab. Itch files vary. | **Per item.** This pass got HTTP 403 from Sketchfab’s license article, so the badge on the model page and the downloaded license are the record. Itch licenses are written by the creator and often allow commercial use while forbidding redistribution of the raw file. |
| **Mixamo** (Adobe) | Preset characters and an auto-rigger. Not a proportion editor. MPFB’s site has a tutorial for animating MPFB characters with Mixamo, which is a workflow note, not a license grant. | Clothing is part of the character, not a separate CC0 wardrobe. | Humanoid FBX, widely used as an animation source. Bone names differ from MakeHuman’s default rig. | Adobe’s Mixamo FAQ page returned HTTP 403 in this pass. **Do not treat Mixamo characters or clips as CC0.** Read the current Adobe terms on the account you download with before any clip is baked into a shipping sheet. |
| **MB-Lab and CharMorph** (Manuel Bastioni lineage) | Strong body sliders, including non-human bases in CharMorph’s design. | Hair and clothes exist inside that ecosystem and share its database license. | Blender-native. CharMorph README describes Rigify at finalize time, and realtime fitting with masks. | `license.txt` in both repos: **code GPL-3.0; mesh database AGPL-3.0; generated 3D models AGPL-3.0** because they are described as derivatives of that database. CharMorph says to use a CC-BY base if that propagation is a problem. The same MB-Lab text says an ordinary **2D render** (not a reverse-engineering turntable of the texture) is a separate work the renderer can license and sell. That is their wording; confirm the rest of `license.txt` before relying on a sprite-only path. Shipping the mesh in a closed glTF client conflicts with the AGPL paragraph they wrote. |
| **SMPL, SMPL-X, and Anny’s `smplx` topology** | Research body models with shape coefficients. | Body only. | Different topology from hm08. | Anny’s README: the SMPL-X topology download is **non-commercial**. Leave this path unused for Scarlet City. |

Blender itself is the DCC for all of the above. `blender.org/about/license` returned HTTP 403 in this pass, so this note does not restate Blender’s GPL output rules. Confirm that page before distributing `.blend` files that still contain add-on data.

## Polygon weight

Documented figures, not a budget measured in this repo:

- MakeHuman **hm08** basemesh: **14766 quad faces**, quads only, “light and optimized for subdivision” (`professional_mesh_topology.html`). Slider changes do not add or remove faces (`concept_basemesh_and_helpers.html`).
- A documented game-density proxy, **`male1591`**, is **1591 faces** (`concept_proxymeshes.html`). Same character, much coarser surface. The page says system proxies can share skins with the basemesh, and that some third-party proxies need their own.

For **sprite baking**, hm08 is a comfortable source mesh. Density only affects Blender. For a **future WebGL client**, start from a proxy in that low-thousands-of-faces class, then look at the skinned result on the actual isometric camera. Hands and faces shrink to a few pixels from the heist camera, so a cinematic mesh spends triangles the player never sees. Count the mesh after export. Helper geometry (joint cubes, skirt helper) should stay out of the shipped mesh.

## Modular clothes, hair, and gear

MakeHuman-family clothes are not interchangeable props. They are surfaces built against a specific body and a set of hidden helpers. Two failure modes show up in the project’s own docs:

- **Proportion changes.** MPFB’s FAQ “After changing a model slider, the clothes no longer fit” points at **Refit assets to basemesh** and **Auto refit assets**. Auto refit gets heavy on dense garments. The asset-pack FAQ adds the artist-side version of the same problem: a piece authored on one body can leave the skin showing through on another when delete-groups are missing or were accepted anyway.
- **Skeleton changes.** Joint cubes are how MPFB places bone heads and tails. A jacket weighted to the MakeHuman skeleton will not follow a Mixamo, Rigify, or Quaternius humanoid until the weights are transferred and the rest pose matches. Anny’s default 104-bone rig is a reduced MakeHuman rig; the README says `rig="makehuman"` is the one that keeps the full 163-bone layout. Hair cards and geometry-nodes hair (called out in the MPFB 2.0.11 news line) need to become a mesh or a baked sprite before the canvas can show them.

Places to look for parts, with the catch attached:

- **System and community MakeHuman packs** when the piece must follow hm08 sliders. Prefer CC0 rows. Record CC-BY credits next to the asset.
- **Quaternius modular outfits** when a stylized humanoid kit is acceptable and the kit file says CC0 or QAL. Expect to retarget, not to drop the jacket onto hm08.
- **Kenney** for blockout silhouettes under CC0.
- **OpenGameArt, Sketchfab, itch.io** one submission at a time. A CC0 filter is a search aid. The file’s license text is the decision.

Hats, masks, and backpacks are the pieces worth making swappable first. They mostly need a socket and a scale check. Jackets, pants, boots, and long hair need a refit and a clipping pass for every archetype that wears them.

## Two pipelines

### Option A — runtime 3D in the webview

Load a glTF body, attach modular meshes or blendshapes, skin them, and draw with WebGL. Yaw stays continuous. Outfit changes are data, not new textures. Altitude is just the mesh’s `z`.

That renderer does not exist. `CanvasRenderer` sends a list of 2D draw ops. A WebGL backend would be a new `IRenderer` plus a JS skinning path, with a budget for every visible player, corpse, and passenger. Anny’s PyTorch model is an offline generator; it is not something to run inside Blazor WASM. Modular glTF also inherits the refit problem: a jacket skinned at one body shape clips when blendshapes change the chest unless the cloth was fitted for that shape or is re-weighted at runtime.

Choose A when the client is already 3D and the wardrobe is small enough to pre-fit per archetype.

### Option B — bake isometric sheets

Pose the finished 3D loadout in Blender and render an atlas: 8 yaw steps, the actions the game actually plays (idle, walk, aim, hit), and the same camera as `IsoProjection`. The canvas blits a sprite. This matches today’s renderer and keeps WASM light.

The cost is combinatorial. Body × hair × jacket × pants × hat × facing × frame becomes a large atlas if every axis is live. Freezing archetypes and baking **loadouts** (a body with a jacket and pants already on) keeps the atlas small. A hat or mask can be a second sprite parented in screen space only while its socket stays stable across those 8 angles. Test that before promising it; a brim that slides off the head at 45° should be baked into the loadout instead.

Yaw will snap to the nearest facing until Option A exists. With mouse aim, 8 directions is the first test, 16 the fallback.

Option B is the starter. Author as if Option A will happen later: one skeleton, named slots, glTF exports archived beside the sheets.

## Test and evaluation checklist

Do this once on a single archetype before building a wardrobe.

1. **Record the license.** Copy the pack page row and the license file from the zip into the asset’s folder. For MPFB core work, that is the CC0 statement. For anything else, stop if the file is AGPL, CC-BY-SA, GPL, or “non-commercial.”
2. **Import the body.** Bring the MPFB human into a fresh Blender file. Apply scale. Confirm a standing character is a plausible human height in meters. MakeHuman’s standalone DAE doc uses a decimeter export scale; do not assume MPFB matches that until measured.
3. **Count the mesh.** Note vertices, faces, and whether triangles or quads. Compare with 14766 quads if the source is hm08, or 1591 faces if the source is the documented `male1591` proxy. Hide helper geometry and joint cubes and confirm they are not in the export.
4. **Import one garment** authored for that body (a system-pack piece such as a fedora or a casual suit is enough). Check that it shares the body’s armature, that bone names match one for one, and that the rest pose lines up at the neck, shoulders, wrists, and ankles.
5. **Move one proportion.** Change a build slider, then run **Refit assets to basemesh**. Orbit the armpit, crotch, collar, and wrist. Look for skin showing through and for cloth intersecting the body. If it fails, that garment is a per-archetype bake, not a global module.
6. **Pose a walk extreme.** Bend shoulders and knees to a walking extreme. Weight painting that looks fine in A-pose often collapses at the elbow. Hair cards should not stab the torso.
7. **Swap the skeleton on purpose.** If a Mixamo or Quaternius clip is part of the test, retarget onto the MakeHuman rig and write down bones that did not map. Do not “fix” this by shipping two skeletons.
8. **Bake eight frames.** Set an orthographic camera whose screen axes match `screenX = (x - y)`, `screenY = (x + y) * 0.5`, and whose vertical shift matches `-z`. Render yaw `0°, 45°, …, 315°`. Drop the sheet into a scratch canvas (or a throwaway image overlay) and check that the feet sit on the same ground point as the current capsule, and that the shadow still reads when `z` increases.
9. **Read the silhouette at game size.** View the sheet at the scale the isometric camera actually uses. If fingers, facial sliders, and cloth seams disappear, spend the next iteration on the read of the jacket, mask, and shoulders.
10. **Decide the slot.** If the hat survived steps 5–8 on two archetypes, it can be a separate sheet. If the jacket survived on only one, bake it into that loadout.

## Sources read for this note

Fetched 25 November 2026 unless marked.

- https://static.makehumancommunity.org/ — forum and asset-repo outage notes (2026-10-21, 2026-11-08)
- https://static.makehumancommunity.org/about/license.html — split license, core assets CC0, MPFB GPL, MakeHuman AGPL
- https://static.makehumancommunity.org/makehuman/faq/are_makehuman_files_free.html
- https://static.makehumancommunity.org/makehuman/faq/can_i_sell_models_created_with_makehuman.html
- https://static.makehumancommunity.org/mpfb/faq/can_i_sell_models.html
- https://static.makehumancommunity.org/mpfb/faq/is_it_really_free.html
- https://static.makehumancommunity.org/mpfb/faq/use_in_closed_source.html
- https://static.makehumancommunity.org/mpfb/faq/build_other_chargen.html
- https://static.makehumancommunity.org/mpfb/faq/clothes_do_not_fit.html
- https://static.makehumancommunity.org/mpfb/docs/assets/concept_basemesh_and_helpers.html
- https://static.makehumancommunity.org/mpfb/docs/assets/concept_proxymeshes.html
- https://static.makehumancommunity.org/makehuman/docs/professional_mesh_topology.html — hm08, 14766 faces
- https://static.makehumancommunity.org/makehuman/docs/exports_and_file_formats.html — DAE scale and axes
- https://static.makehumancommunity.org/assets/assetpacks/faq.html — CC0 vs CC-BY, body-fit warning
- https://static.makehumancommunity.org/assets/assetpacks/makehuman_system_assets.html — CC0 labels on listed system assets (page not fully transcribed)
- https://github.com/makehumancommunity/makehuman/blob/master/LICENSE.md
- https://github.com/makehumancommunity/mpfb2/blob/master/LICENSE.ASSETS.md
- https://github.com/naver/anny — README license section and rig/topology notes; `LICENSE` (Apache-2.0)
- https://quaternius.com/license.html — QAL v1.0 (8/28/2026)
- https://quaternius.com/faq.html — per-kit CC0 or QAL
- https://quaternius.com/ — catalog tags for Universal Base Characters and Modular Character Outfits - Fantasy
- https://kenney.nl/support — CC0 statement for asset pages
- https://opengameart.org/content/faq — commercial use only when the submission license is followed
- https://raw.githubusercontent.com/animate1978/MB-Lab/master/license.txt
- https://raw.githubusercontent.com/Upliner/CharMorph/master/license.txt and `README.md`

Not retrieved (HTTP 403): `blender.org/about/license`, Sketchfab’s license article, Adobe’s Mixamo help page. Those three stay unverified on purpose.
