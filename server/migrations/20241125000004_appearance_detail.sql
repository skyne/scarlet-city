UPDATE crew_members
SET appearance_json = json_patch(
  '{"body_set":"male","hair":"buzzed","outfit":"peasant","beard":false,"brows":true,"hair_color":"#2a1b14","beard_color":"#2a1b14","eye_color":"#6b4a2f","skin_color":"#d8a07a","outfit_color":"#ffffff","height":0.5,"head":0.5,"neck":0.5,"shoulders":0.5,"bust":0.35,"waist":0.5,"hips":0.5,"bum":0.35,"muscle":0.35,"arms":0.5,"legs":0.5,"leg_length":0.5,"torso_length":0.5}',
  appearance_json
)
WHERE json_valid(appearance_json);

UPDATE crew_members
SET appearance_json = '{"body_set":"male","hair":"buzzed","outfit":"peasant","beard":false,"brows":true,"hair_color":"#2a1b14","beard_color":"#2a1b14","eye_color":"#6b4a2f","skin_color":"#d8a07a","outfit_color":"#ffffff","height":0.5,"head":0.5,"neck":0.5,"shoulders":0.5,"bust":0.35,"waist":0.5,"hips":0.5,"bum":0.35,"muscle":0.35,"arms":0.5,"legs":0.5,"leg_length":0.5,"torso_length":0.5}'
WHERE NOT json_valid(appearance_json);
