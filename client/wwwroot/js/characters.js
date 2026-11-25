import * as THREE from "three";
import { GLTFLoader } from "./vendor/loaders/GLTFLoader.js";
import { clone as cloneSkinned } from "./vendor/utils/SkeletonUtils.js";

const TARGET_HEIGHT = 1.7;
const YAW_OFFSET = -Math.PI / 2;
const OUTFIT_IDS = ["peasant", "ranger", "casual", "suit", "worker", "punk", "tactical"];
const CLOTHING_SLOT_IDS = ["none", ...OUTFIT_IDS];
const ACCESSORY_SLOT_IDS = ["none", "ranger"];
const WEAPON_IDS = ["none", "pistol", "revolver", "shotgun", "smg", "rifle", "sniper", "bullpup"];
const WEAPON_ASSETS = {
  pistol: "assets/world/models/quaternius/guns/Pistol.glb",
  revolver: "assets/world/models/quaternius/guns/Revolver.glb",
  shotgun: "assets/world/models/quaternius/guns/Shotgun.glb",
  smg: "assets/world/models/quaternius/guns/Submachine_Gun.glb",
  rifle: "assets/world/models/quaternius/guns/Assault_Rifle.glb",
  sniper: "assets/world/models/quaternius/guns/Sniper_Rifle.glb",
  bullpup: "assets/world/models/quaternius/guns/Bullpup.glb"
};
// Fits are in character-local meters, compensated by rec.baseScale at attach time.
const WEAPON_FITS = {
  pistol:   { s: 0.18, pos: [0.02, 0.08, 0.03], rot: [0, Math.PI / 2, Math.PI / 2] },
  revolver: { s: 0.18, pos: [0.02, 0.08, 0.03], rot: [0, Math.PI / 2, Math.PI / 2] },
  shotgun:  { s: 0.22, pos: [0.02, 0.12, 0.03], rot: [0, Math.PI / 2, Math.PI / 2] },
  smg:      { s: 0.20, pos: [0.02, 0.10, 0.03], rot: [0, Math.PI / 2, Math.PI / 2] },
  rifle:    { s: 0.22, pos: [0.02, 0.12, 0.03], rot: [0, Math.PI / 2, Math.PI / 2] },
  sniper:   { s: 0.24, pos: [0.02, 0.14, 0.03], rot: [0, Math.PI / 2, Math.PI / 2] },
  bullpup:  { s: 0.21, pos: [0.02, 0.11, 0.03], rot: [0, Math.PI / 2, Math.PI / 2] }
};
const HAIR_TINT_MATERIALS = new Set(["MI_Hair_1", "MI_Hair_2", "Hair", "Hair_Blond", "Hair_Brown"]);
const OUTFIT_TINT_MATERIALS = new Set([
  "Purple", "LightBlue", "Suit", "Tie", "Worker_Yellow", "Worker_Vest",
  "Swat", "Swat_Black", "Pink", "Orange"
]);

const loader = new GLTFLoader();
const clock = new THREE.Clock();

const state = {
  ready: false,
  error: null,
  renderer: null,
  scene: null,
  camera: null,
  canvas: null,
  view: new THREE.Matrix4(),
  sets: [],
  clips: {},
  weapons: {},
  live: new Map()
};

const preview = { scene: null, camera: null, rec: null, key: "", yaw: 0, dragging: false, lastX: 0, appearance: null };

const DEFAULT_APPEARANCE = {
  body_set: "male",
  hair: "buzzed",
  outfit: "peasant",
  beard: false,
  brows: true,
  hair_color: "#2a1b14",
  beard_color: "#2a1b14",
  eye_color: "#6b4a2f",
  skin_color: "#d8a07a",
  outfit_color: "#ffffff",
  height: 0.5,
  head: 0.5,
  neck: 0.5,
  shoulders: 0.5,
  bust: 0.35,
  waist: 0.5,
  hips: 0.5,
  bum: 0.35,
  muscle: 0.35,
  arms: 0.5,
  legs: 0.5,
  leg_length: 0.5,
  torso_length: 0.5,
  slot_top: "peasant",
  slot_bottom: "peasant",
  slot_shoes: "peasant",
  slot_gloves: "peasant",
  slot_head: "none",
  slot_shoulder: "none",
  slot_belt: "none",
  weapon: "pistol"
};

function clamp01(v, fallback = 0.5) {
  const n = Number(v);
  return Number.isFinite(n) ? Math.max(0, Math.min(1, n)) : fallback;
}

function normSlot(v, oldWear, on, off, accessory = false) {
  const allowed = accessory ? ACCESSORY_SLOT_IDS : CLOTHING_SLOT_IDS;
  if (allowed.includes(v)) return v;
  if (typeof oldWear === "boolean") return oldWear ? on : "none";
  return off;
}

function normAppearance(raw) {
  const a = raw && typeof raw === "object" ? raw : {};
  const body = a.body_set === "female" ? "female" : "male";
  const hairIds = body === "female"
    ? ["buzzed_female", "buns", "long", "short_street"]
    : ["buzzed", "simple_parted", "long", "short_street"];
  const outfitIds = OUTFIT_IDS;
  const beard = body !== "female" && !!a.beard;
  let hair = hairIds.includes(a.hair) ? a.hair : hairIds[0];
  if (a.hair === "beard") hair = hairIds[0];
  const outfit = outfitIds.includes(a.outfit) ? a.outfit : outfitIds[0];
  return {
    body_set: body,
    hair,
    outfit,
    beard,
    brows: a.brows !== false,
    hair_color: typeof a.hair_color === "string" ? a.hair_color : DEFAULT_APPEARANCE.hair_color,
    beard_color: typeof a.beard_color === "string" ? a.beard_color : (typeof a.hair_color === "string" ? a.hair_color : DEFAULT_APPEARANCE.beard_color),
    eye_color: typeof a.eye_color === "string" ? a.eye_color : DEFAULT_APPEARANCE.eye_color,
    skin_color: typeof a.skin_color === "string" ? a.skin_color : DEFAULT_APPEARANCE.skin_color,
    outfit_color: typeof a.outfit_color === "string" ? a.outfit_color : DEFAULT_APPEARANCE.outfit_color,
    height: clamp01(a.height),
    head: clamp01(a.head),
    neck: clamp01(a.neck),
    shoulders: clamp01(a.shoulders),
    bust: clamp01(a.bust, body === "female" ? 0.5 : 0.35),
    waist: clamp01(a.waist),
    hips: clamp01(a.hips),
    bum: clamp01(a.bum, body === "female" ? 0.45 : 0.35),
    muscle: clamp01(a.muscle, 0.35),
    arms: clamp01(a.arms),
    legs: clamp01(a.legs),
    leg_length: clamp01(a.leg_length),
    torso_length: clamp01(a.torso_length),
    slot_top: normSlot(a.slot_top, a.wear_top, outfit, "peasant"),
    slot_bottom: normSlot(a.slot_bottom, a.wear_bottom, outfit, "peasant"),
    slot_shoes: normSlot(a.slot_shoes, a.wear_shoes, outfit, "peasant"),
    slot_gloves: normSlot(a.slot_gloves, a.wear_gloves, outfit, "peasant"),
    slot_head: normSlot(a.slot_head, a.wear_head, outfit === "ranger" ? "ranger" : "none", "none", true),
    slot_shoulder: normSlot(a.slot_shoulder, a.wear_shoulder, outfit === "ranger" ? "ranger" : "none", "none", true),
    slot_belt: normSlot(a.slot_belt, a.wear_belt, outfit === "ranger" ? "ranger" : "none", "none", true),
    weapon: WEAPON_IDS.includes(a.weapon) ? a.weapon : "pistol"
  };
}

function apKey(a) {
  return [
    a.body_set, a.hair, a.outfit, a.beard ? 1 : 0, a.brows ? 1 : 0,
    a.hair_color, a.beard_color, a.eye_color, a.skin_color, a.outfit_color,
    a.height, a.head, a.neck, a.shoulders, a.bust, a.waist, a.hips, a.bum, a.muscle, a.arms, a.legs, a.leg_length, a.torso_length,
    a.slot_top, a.slot_bottom, a.slot_shoes, a.slot_gloves, a.slot_head, a.slot_shoulder, a.slot_belt, a.weapon
  ].join("|");
}

function cloneMat(material) {
  if (Array.isArray(material)) return material.map((m) => m?.clone ? m.clone() : m);
  return material?.clone ? material.clone() : material;
}

function safeColor(color, fallback) {
  if (typeof color !== "string" || !/^#[0-9a-fA-F]{6}$/.test(color)) return new THREE.Color(fallback);
  return new THREE.Color(color);
}

function tintMat(mat, color) {
  if (!mat?.color) return;
  if (typeof color !== "string" || !/^#[0-9a-fA-F]{6}$/.test(color) || color.toLowerCase() === "#ffffff") return;
  mat.color.multiply(safeColor(color, "#ffffff"));
}

function shapeBone(rec, name, sx, sy, sz) {
  rec.boneScales[name] = [sx, sy, sz];
}

function applyBoneScales(rec) {
  const bones = rec.rig.bones;
  if (!bones) return;
  for (const [name, s] of Object.entries(rec.boneScales)) {
    const bone = bones[name];
    if (bone) bone.scale.set(s[0], s[1], s[2]);
  }
}

function applyAppearance(rec) {
  const a = rec.appearance;
  rec.boneScales = {};
  rec.rig.scale.setScalar(rec.baseScale * (0.92 + a.height * 0.18));
  rec.rig.updateMatrixWorld(true);
  const fit = new THREE.Box3().setFromObject(rec.rig);
  rec.rig.position.z -= fit.min.z;
  for (const mesh of rec.meshes) {
    const mats = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
    for (const mat of mats) {
      if (!mat) continue;
      if (mesh.userData.kind === "hair" && HAIR_TINT_MATERIALS.has(mat.name)) tintMat(mat, a.hair_color);
      if (mesh.userData.kind === "beard" && HAIR_TINT_MATERIALS.has(mat.name)) tintMat(mat, a.beard_color);
      if (mesh.userData.kind === "brows" && (mat.name?.startsWith("MI_Hair") || HAIR_TINT_MATERIALS.has(mat.name))) tintMat(mat, a.hair_color);
      if (mesh.userData.kind === "body" && mat.name === "MI_Eyes") tintMat(mat, a.eye_color);
      if (mesh.userData.kind === "outfit" && (mat.name === "MI_Ranger" || OUTFIT_TINT_MATERIALS.has(mat.name))) tintMat(mat, a.outfit_color);
    }
  }

  const head = 0.9 + a.head * 0.22;
  shapeBone(rec, "Head", head, head, head);
  const neckLen = 0.88 + a.neck * 0.28;
  const neckThick = 0.95 + a.muscle * 0.08;
  shapeBone(rec, "neck_01", neckThick, neckLen, neckThick);

  // Shoulders: only clavicle offset; do not scale spine.
  const shoulder = 0.04 + a.shoulders * 0.14;
  const cl = rec.rig.bones.clavicle_l, cr = rec.rig.bones.clavicle_r;
  if (cl) cl.position.x = (cl.userData.baseX ?? (cl.userData.baseX = cl.position.x)) - shoulder;
  if (cr) cr.position.x = (cr.userData.baseX ?? (cr.userData.baseX = cr.position.x)) + shoulder;

  const arm = 0.9 + a.arms * 0.22 + a.muscle * 0.08;
  shapeBone(rec, "upperarm_l", arm, 1, arm);
  shapeBone(rec, "upperarm_r", arm, 1, arm);
  shapeBone(rec, "lowerarm_l", 0.94 + a.arms * 0.16, 1, 0.94 + a.arms * 0.16);
  shapeBone(rec, "lowerarm_r", 0.94 + a.arms * 0.16, 1, 0.94 + a.arms * 0.16);

  const torsoLen = 0.94 + a.torso_length * 0.12;
  const waist = 0.88 + a.waist * 0.24;
  const hip = 0.88 + a.hips * 0.28;
  // Pelvis scales hips; compensate only immediate child spine_01 on X so upper body is independent.
  const invHipX = 1 / hip;
  shapeBone(rec, "spine_01", waist * invHipX, torsoLen, waist);
  shapeBone(rec, "spine_02", 0.92 + a.waist * 0.16, torsoLen, 0.92 + a.waist * 0.16);
  const bust = Math.max(0, Math.min(1, a.bust));
  const bustZ = 1 + bust * 0.35;
  const bustX = 1 + bust * 0.08;
  shapeBone(rec, "spine_03", (0.96 + a.muscle * 0.06) * bustX, torsoLen, (0.96 + a.muscle * 0.06) * bustZ);
  const invBustX = 1 / bustX, invBustZ = 1 / bustZ;
  shapeBone(rec, "clavicle_l", invBustX, 1, invBustZ);
  shapeBone(rec, "clavicle_r", invBustX, 1, invBustZ);
  shapeBone(rec, "neck_01", neckThick * invBustX, neckLen, neckThick * invBustZ);
  shapeBone(rec, "Head", head * invBustX, head, head * invBustZ);

  const bum = 0.9 + a.bum * 0.24;
  shapeBone(rec, "pelvis", hip, 1, 0.95 + a.bum * 0.16);
  const thigh = 0.9 + a.legs * 0.22;
  const legLen = 0.94 + a.leg_length * 0.12;
  shapeBone(rec, "thigh_l", thigh, legLen, thigh + (bum - 1) * 0.25);
  shapeBone(rec, "thigh_r", thigh, legLen, thigh + (bum - 1) * 0.25);
  const calf = 0.92 + a.legs * 0.18;
  shapeBone(rec, "calf_l", calf, legLen, calf);
  shapeBone(rec, "calf_r", calf, legLen, calf);
  applyBoneScales(rec);
  applyOutfitVisibility(rec);
}

function ensure() {
  if (state.renderer) return;
  const canvas = document.createElement("canvas");
  const renderer = new THREE.WebGLRenderer({
    canvas,
    alpha: true,
    antialias: true,
    premultipliedAlpha: false
  });
  renderer.setClearColor(0x000000, 0);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  const scene = new THREE.Scene();
  scene.add(new THREE.HemisphereLight(0xfff4ea, 0x241614, 0.9));
  const sun = new THREE.DirectionalLight(0xfff7ee, 1.45);
  sun.position.set(-6, -8, 12);
  scene.add(sun);
  const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 8000);
  camera.matrixAutoUpdate = false;
  state.canvas = canvas;
  state.renderer = renderer;
  state.scene = scene;
  state.camera = camera;
}

function loadGltf(url) {
  return loader.loadAsync(url);
}

function prepareTemplate(sceneRoot) {
  const rig = new THREE.Group();
  sceneRoot.rotation.x = Math.PI / 2;
  rig.add(sceneRoot);
  rig.updateMatrixWorld(true);
  const box = new THREE.Box3().setFromObject(rig);
  const height = Math.max(0.001, box.max.z - box.min.z);
  rig.scale.setScalar(TARGET_HEIGHT / height);
  rig.updateMatrixWorld(true);
  const fitted = new THREE.Box3().setFromObject(rig);
  rig.position.z -= fitted.min.z;
  rig.updateMatrixWorld(true);
  return rig;
}

function pickClip(names) {
  for (const name of names) {
    if (state.clips[name]) return state.clips[name];
  }
  return null;
}

function play(rec, speed) {
  const armed = rec.appearance?.weapon && rec.appearance.weapon !== "none";
  let next = speed > 4.5 ? rec.sprint : speed > 0.35 ? rec.jog : rec.idle;
  if (armed && rec.armedIdle && speed <= 0.35) next = rec.armedIdle;
  if (!next || next === rec.current) return;
  next.reset().fadeIn(0.15).play();
  if (rec.current) rec.current.fadeOut(0.15);
  rec.current = next;
}

function attachWeapon(rec) {
  const id = rec.appearance?.weapon;
  if (!id || id === "none") return;
  const tpl = state.weapons[id];
  const hand = rec.rig.bones?.hand_r;
  if (!tpl || !hand) return;
  const obj = cloneSkinned(tpl.scene || tpl);
  const fit = WEAPON_FITS[id] || WEAPON_FITS.pistol;
  const scale = fit.s / (rec.baseScale || 1);
  obj.scale.setScalar(scale);
  obj.position.set(fit.pos[0] / (rec.baseScale || 1), fit.pos[1] / (rec.baseScale || 1), fit.pos[2] / (rec.baseScale || 1));
  obj.rotation.set(fit.rot[0], fit.rot[1], fit.rot[2]);
  obj.traverse((n) => { if (n.isMesh) { n.castShadow = false; n.receiveShadow = false; n.frustumCulled = false; } });
  hand.add(obj);
  rec.weaponObj = obj;
}

function spawn(actor, scene = state.scene) {
  const appearance = normAppearance(actor.appearance || DEFAULT_APPEARANCE);
  const bodyIndex = appearance.body_set === "female" ? 1 : 0;
  const set = state.sets[bodyIndex] || state.sets[0];
  const rig = cloneSkinned(set.body);
  const holder = new THREE.Group();
  holder.add(rig);
  rig.bones = {};
  rig.traverse((obj) => { if (obj.isBone) { obj.scale.set(1, 1, 1); rig.bones[obj.name] = obj; } });

  const meshes = [];
  const tag = (root, kind) => root.traverse((obj) => {
    if (obj.isMesh || obj.isSkinnedMesh) { obj.material = cloneMat(obj.material); obj.userData.kind = kind; meshes.push(obj); }
  });
  tag(rig, "body");

  const hair = set.hairs[set.hairIds.indexOf(appearance.hair)] || set.hairs[0];
  attachPart(hair, rig, "hair", meshes);
  if (appearance.brows && set.brows) attachPart(set.brows, rig, "brows", meshes);
  if (appearance.beard && set.beard) attachPart(set.beard, rig, "beard", meshes);
  const worn = new Set([
    appearance.slot_top, appearance.slot_bottom, appearance.slot_shoes, appearance.slot_gloves,
    appearance.slot_head, appearance.slot_shoulder, appearance.slot_belt
  ]);
  for (let i = 0; i < (set.outfits?.length || 0); i++) {
    const src = set.outfitIds[i] || (i === 0 ? "peasant" : "ranger");
    if (!worn.has(src)) continue;
    attachPart(set.outfits[i], rig, "outfit", meshes, src);
  }

  scene.add(holder);
  const mixer = new THREE.AnimationMixer(rig);
  const action = (clip) => (clip ? mixer.clipAction(clip, rig) : null);
  const rec = {
    holder,
    rig,
    meshes,
    baseScale: rig.scale.x,
    appearance,
    apKey: apKey(appearance),
    variant: actor.variant,
    boneScales: {},
    mixer,
    idle: action(pickClip(["Idle_Loop", "Idle"])),
    jog: action(pickClip(["Jog_Fwd_Loop", "Walk_Loop", "Walk"])),
    sprint: action(pickClip(["Sprint_Loop", "Jog_Fwd_Loop"])),
    armedIdle: action(pickClip(["Pistol_Idle_Loop", "Pistol_Aim_Neutral"])),
    current: null
  };
  applyBodyMask(rec);
  attachWeapon(rec);
  applyAppearance(rec);
  if (rec.idle) {
    rec.idle.play();
    rec.current = rec.idle;
  }
  return rec;
}

function skinParent(root) {
  let parent = root;
  root.traverse((obj) => {
    if (obj.isSkinnedMesh && obj.parent) parent = obj.parent;
  });
  return parent;
}

const SLOT_KEYS = ["slot_top", "slot_bottom", "slot_shoes", "slot_gloves", "slot_head", "slot_shoulder", "slot_belt"];

const TOP_BONES = new Set([
  "spine_01", "spine_02", "spine_03", "clavicle_l", "clavicle_r",
  "upperarm_l", "upperarm_r", "lowerarm_l", "lowerarm_r"
]);
const BOTTOM_BONES = new Set(["pelvis", "thigh_l", "thigh_r", "calf_l", "calf_r"]);
const SHOE_BONES = new Set(["foot_l", "foot_r", "ball_l", "ball_r", "ball_leaf_l", "ball_leaf_r"]);

function boneRegion(name) {
  if (TOP_BONES.has(name)) return "top";
  if (BOTTOM_BONES.has(name)) return "bottom";
  if (SHOE_BONES.has(name)) return "shoes";
  return "keep";
}

function attrComponent(attr, i, c) {
  if (attr.getComponent) return attr.getComponent(i, c);
  if (attr.isInterleavedBufferAttribute) return attr.data.array[attr.offset + i * attr.data.stride + c];
  return attr.array[i * attr.itemSize + c];
}

function dominantBone(skinIndex, skinWeight, i) {
  let best = 0;
  let bestWeight = -1;
  for (let c = 0; c < 4; c++) {
    const w = attrComponent(skinWeight, i, c);
    if (w > bestWeight) { bestWeight = w; best = attrComponent(skinIndex, i, c); }
  }
  return best;
}

function applyBodyMask(rec) {
  const hidden = new Set();
  if (rec.appearance.slot_top !== "none") hidden.add("top");
  if (rec.appearance.slot_bottom !== "none") hidden.add("bottom");
  if (rec.appearance.slot_shoes !== "none") hidden.add("shoes");
  if (!hidden.size) return;
  const boneNames = (mesh) => (mesh.skeleton?.bones || Object.values(rec.rig.bones || {})).map((b) => b?.name);
  for (const mesh of rec.meshes) {
    if (mesh.userData.kind !== "body") continue;
    if (!mesh.userData.baseGeometry) {
      mesh.userData.baseGeometry = mesh.geometry.index ? mesh.geometry.toNonIndexed() : mesh.geometry.clone();
    }
    const base = mesh.userData.baseGeometry;
    const skinIndex = base.getAttribute("skinIndex");
    const skinWeight = base.getAttribute("skinWeight");
    const position = base.getAttribute("position");
    if (!skinIndex || !skinWeight || !position) continue;
    const names = boneNames(mesh);
    const keep = [];
    for (let i = 0; i < position.count; i += 3) {
      let drop = false;
      for (let k = 0; k < 3; k++) {
        const bi = dominantBone(skinIndex, skinWeight, i + k);
        if (hidden.has(boneRegion(names[bi]))) { drop = true; break; }
      }
      if (!drop) keep.push(i, i + 1, i + 2);
    }
    if (keep.length === position.count) continue;
    const geometry = new THREE.BufferGeometry();
    for (const [name, attr] of Object.entries(base.attributes)) {
      const itemSize = attr.itemSize || 1;
      const array = new attr.array.constructor(keep.length * itemSize);
      for (let d = 0; d < keep.length; d++) {
        for (let c = 0; c < itemSize; c++) array[d * itemSize + c] = attrComponent(attr, keep[d], c);
      }
      geometry.setAttribute(name, new THREE.BufferAttribute(array, itemSize, attr.normalized));
    }
    mesh.geometry = geometry;
  }
}

function slotForMesh(meshName) {
  const n = String(meshName || "").toLowerCase();
  if (n.includes("hood") || n.startsWith("head_")) return "slot_head";
  if (n.includes("pauldron") || n.includes("shoulder")) return "slot_shoulder";
  if (n.includes("bracer") || n.includes("glove") || n.includes("arm")) return "slot_gloves";
  if (n.includes("belt") || n.includes("waist")) return "slot_belt";
  if (n.includes("boot") || n.includes("shoe") || n.includes("foot") || n.includes("feet")) return "slot_shoes";
  if (n.includes("leg") || n.includes("pant")) return "slot_bottom";
  if (n.includes("body") || n.includes("chest") || n.includes("torso") || n.includes("top")) return "slot_top";
  return "slot_top";
}

function meshLabel(name) {
  return String(name || "piece").replace(/^(female|male)_/i, "").replace(/_/g, " ");
}

function applyOutfitVisibility(rec) {
  const a = rec.appearance;
  for (const mesh of rec.meshes) {
    if (mesh.userData.kind !== "outfit") continue;
    const slot = mesh.userData.slot || slotForMesh(mesh.name);
    const src = mesh.userData.outfitSource;
    mesh.visible = !!src && a[slot] === src;
  }
}

function attachPart(template, host, kind, outMeshes, outfitSource = null) {
  const part = cloneSkinned(template);
  const bones = new Map();
  host.traverse((obj) => {
    if (obj.isBone) bones.set(obj.name, obj);
  });
  const parent = skinParent(host);
  const meshes = [];
  part.traverse((obj) => {
    if (obj.isSkinnedMesh) meshes.push(obj);
  });
  for (const obj of meshes) {
    const source = obj.skeleton.bones;
    const mapped = source.map((bone) => bones.get(bone.name) || bone);
    const hits = mapped.filter((bone, i) => bone !== source[i]).length;
    if (hits < source.length * 0.5) continue;
    obj.material = cloneMat(obj.material);
    obj.skeleton = new THREE.Skeleton(mapped, obj.skeleton.boneInverses);
    obj.bind(obj.skeleton, obj.bindMatrix);
    obj.userData.kind = kind;
    if (kind === "outfit") {
      obj.userData.outfitSource = outfitSource;
      obj.userData.slot = slotForMesh(obj.name);
    }
    if (outMeshes) outMeshes.push(obj);
    parent.add(obj);
  }
}

function sync(actors, dt) {
  const seen = new Set();
  for (const actor of actors || []) {
    seen.add(actor.id);
    const appearance = actor.appearance || DEFAULT_APPEARANCE;
    const key = apKey(normAppearance(appearance));
    let rec = state.live.get(actor.id);
    if (rec && rec.apKey !== key) {
      state.scene.remove(rec.holder);
      state.live.delete(actor.id);
      rec = null;
    }
    if (!rec) {
      rec = spawn(actor);
      state.live.set(actor.id, rec);
    }
    rec.holder.position.set(actor.x, actor.y, actor.z);
    rec.holder.rotation.z = -actor.yaw - YAW_OFFSET;
    play(rec, actor.speed || 0);
    rec.mixer.update(dt);
    applyBoneScales(rec);
  }
  for (const [id, rec] of state.live) {
    if (seen.has(id)) continue;
    state.scene.remove(rec.holder);
    state.live.delete(id);
  }
}

function previewScene() {
  if (preview.scene) return;
  const scene = new THREE.Scene();
  scene.add(new THREE.HemisphereLight(0xfff4ea, 0x241614, 1.1));
  const sun = new THREE.DirectionalLight(0xfff7ee, 1.6);
  sun.position.set(-3, -5, 7);
  scene.add(sun);
  const camera = new THREE.OrthographicCamera(-1.3, 1.3, 1.8, -1.8, 0.1, 100);
  camera.up.set(0, 0, 1);
  camera.position.set(2.6, -4.2, 2.3);
  camera.lookAt(0, 0, 0.85);
  preview.scene = scene;
  preview.camera = camera;
}

function previewCharacter(canvas, appearance) {
  if (!state.ready || !canvas) return;
  previewScene();
  if (preview.canvas !== canvas) {
    preview.canvas = canvas;
    canvas.style.cursor = "grab";
    if (!canvas.dataset.rot) {
      canvas.dataset.rot = "1";
      canvas.addEventListener("pointerdown", (e) => {
        preview.dragging = true;
        preview.lastX = e.clientX;
        canvas.setPointerCapture(e.pointerId);
        canvas.style.cursor = "grabbing";
      });
      canvas.addEventListener("pointermove", (e) => {
        if (!preview.dragging) return;
        preview.yaw += (e.clientX - preview.lastX) * 0.01;
        preview.lastX = e.clientX;
      });
      const end = () => { preview.dragging = false; canvas.style.cursor = "grab"; };
      canvas.addEventListener("pointerup", end);
      canvas.addEventListener("pointerleave", end);
      canvas.addEventListener("pointercancel", end);
    }
  }

  const w = canvas.clientWidth || 360;
  const h = canvas.clientHeight || 520;
  if (canvas.width !== w || canvas.height !== h) {
    canvas.width = w;
    canvas.height = h;
  }
  const a = normAppearance(appearance);
  const key = apKey(a);
  if (preview.key !== key) {
    if (preview.rec) preview.scene.remove(preview.rec.holder);
    preview.rec = spawn({ appearance: a, variant: 0, id: "preview" }, preview.scene);
    preview.rec.holder.rotation.z = YAW_OFFSET + preview.yaw;
    preview.key = key;
  }
  preview.rec.holder.rotation.z = YAW_OFFSET + preview.yaw;
  preview.rec.mixer.update(Math.min(0.05, clock.getDelta()));
  applyBoneScales(preview.rec);

  preview.rec.holder.updateMatrixWorld(true);
  const box = new THREE.Box3().setFromObject(preview.rec.holder);
  const center = box.getCenter(new THREE.Vector3());
  const width = Math.max(0.1, Math.max(box.max.x - box.min.x, box.max.y - box.min.y));
  const aspect = w / Math.max(1, h);
  const viewH = 2.05;
  const viewW = Math.max(width * 1.15, viewH * aspect);
  preview.camera.left = -viewW / 2;
  preview.camera.right = viewW / 2;
  preview.camera.top = viewH / 2;
  preview.camera.bottom = -viewH / 2;
  preview.camera.near = 0.1;
  preview.camera.far = 100;
  preview.camera.updateProjectionMatrix();
  preview.camera.position.set(center.x + 2.8, center.y - 4.4, box.min.z + 1.02);
  preview.camera.lookAt(center.x, center.y, box.min.z + 0.98);

  state.renderer.setSize(w, h, false);
  state.renderer.render(preview.scene, preview.camera);
  const ctx = canvas.getContext("2d");
  ctx.clearRect(0, 0, w, h);
  ctx.drawImage(state.canvas, 0, 0);
}

function frameCamera(canvas, op) {
  const [originX, originY, scale, zScale] = op.pts;
  const w = canvas.width;
  const h = canvas.height;
  const s = scale;
  const z = zScale;
  state.view.set(
    s, -s, 0, originX - w / 2,
    -0.5 * s, -0.5 * s, z, -(originY - h / 2),
    -s, -s, -0.2 * s, -2500,
    0, 0, 0, 1
  );
  state.camera.matrix.copy(state.view).invert();
  state.camera.matrixWorldNeedsUpdate = true;
  state.camera.projectionMatrix.makeOrthographic(-w / 2, w / 2, h / 2, -h / 2, 0.1, 8000);
  state.camera.projectionMatrixInverse.copy(state.camera.projectionMatrix).invert();
  state.renderer.setSize(w, h, false);
}

async function boot() {
  ensure();
  const manifest = await (await fetch("assets/quaternius/manifest.json")).json();
  if (!manifest.sets?.length) throw new Error("no character sets in manifest");
  for (const set of manifest.sets) {
    const body = prepareTemplate((await loadGltf(set.body)).scene);
    const hairs = [];
    for (const url of set.hairs) hairs.push((await loadGltf(url)).scene);
    const brows = set.brows ? (await loadGltf(set.brows)).scene : null;
    const outfits = [];
    for (const url of set.outfits) outfits.push((await loadGltf(url)).scene);
    const beardUrl = set.beard || (set.hairs || []).find((u) => /beard/i.test(u));
    state.sets.push({
      body,
      hairs,
      brows,
      outfits,
      beard: beardUrl ? (await loadGltf(beardUrl)).scene : null,
      hairIds: (set.hair_ids || []).map((s) => String(s)),
      outfitIds: (set.outfit_ids || []).map((s) => String(s))
    });
  }
  // Fallback ids for existing manifest
  if (state.sets[0] && !state.sets[0].hairIds.length) state.sets[0].hairIds = ["buzzed", "simple_parted", "long", "beard"];
  if (state.sets[1] && !state.sets[1].hairIds.length) state.sets[1].hairIds = ["buzzed_female", "buns", "long"];
  for (const s of state.sets) if (!s.outfitIds.length) s.outfitIds = ["peasant", "ranger"];
  if (manifest.animations) {
    const gltf = await loadGltf(manifest.animations);
    for (const clip of gltf.animations || []) state.clips[clip.name] = clip;
  }
  for (const [id, url] of Object.entries(WEAPON_ASSETS)) {
    try { state.weapons[id] = (await loadGltf(url)).scene; }
    catch (e) { console.warn("weapon load failed", id, e); }
  }
  state.ready = true;
}

boot().catch((err) => {
  state.error = String(err && err.stack || err);
  console.error("Scarlet characters failed to load", err);
});

window.ScarletCharacters = {
  get ready() { return state.ready; },
  get error() { return state.error || null; },
  preview(canvasId, appearance) {
    previewCharacter(document.getElementById(canvasId), appearance);
  },
  resetPreviewYaw() { preview.yaw = 0; },
  debugPreview() {
    const r = preview.rec;
    if (!r) return null;
    const bone = (n) => { const b=r.rig.bones?.[n]; return b ? [b.scale.x,b.scale.y,b.scale.z] : null; };
    return { key:r.apKey, shoulders:bone("clavicle_l"), bust:bone("spine_03"), hips:bone("pelvis"), height:r.rig.scale.x/r.baseScale };
  },
  render(ctx, canvas, op) {
    if (!state.ready) return;
    try {
      const dt = Math.min(0.05, clock.getDelta());
      sync(op.actors, dt);
      frameCamera(canvas, op);
      state.renderer.render(state.scene, state.camera);
      ctx.drawImage(state.canvas, 0, 0);
    } catch (err) {
      state.error = String(err && err.stack || err);
      console.error(err);
    }
  }
};
