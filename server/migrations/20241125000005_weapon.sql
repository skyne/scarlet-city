UPDATE crew_members
SET appearance_json = json_patch(
  '{"weapon":"pistol"}',
  appearance_json
)
WHERE json_valid(appearance_json) AND json_extract(appearance_json, '$.weapon') IS NULL;
