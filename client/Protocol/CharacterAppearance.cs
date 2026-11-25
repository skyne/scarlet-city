// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v7

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;


namespace ScarletCity.Protocol;

public sealed class CharacterAppearance
{
    [JsonPropertyName("body_set")]
    public string BodySet { get; set; } = string.Empty;

    [JsonPropertyName("hair")]
    public string Hair { get; set; } = string.Empty;

    [JsonPropertyName("outfit")]
    public string Outfit { get; set; } = string.Empty;

    [JsonPropertyName("beard")]
    public bool Beard { get; set; }

    [JsonPropertyName("brows")]
    public bool Brows { get; set; }

    [JsonPropertyName("hair_color")]
    public string HairColor { get; set; } = string.Empty;

    [JsonPropertyName("beard_color")]
    public string BeardColor { get; set; } = string.Empty;

    [JsonPropertyName("eye_color")]
    public string EyeColor { get; set; } = string.Empty;

    [JsonPropertyName("skin_color")]
    public string SkinColor { get; set; } = string.Empty;

    [JsonPropertyName("outfit_color")]
    public string OutfitColor { get; set; } = string.Empty;

    [JsonPropertyName("height")]
    public float Height { get; set; }

    [JsonPropertyName("head")]
    public float Head { get; set; }

    [JsonPropertyName("neck")]
    public float Neck { get; set; }

    [JsonPropertyName("shoulders")]
    public float Shoulders { get; set; }

    [JsonPropertyName("bust")]
    public float Bust { get; set; }

    [JsonPropertyName("waist")]
    public float Waist { get; set; }

    [JsonPropertyName("hips")]
    public float Hips { get; set; }

    [JsonPropertyName("bum")]
    public float Bum { get; set; }

    [JsonPropertyName("muscle")]
    public float Muscle { get; set; }

    [JsonPropertyName("arms")]
    public float Arms { get; set; }

    [JsonPropertyName("legs")]
    public float Legs { get; set; }

    [JsonPropertyName("leg_length")]
    public float LegLength { get; set; }

    [JsonPropertyName("torso_length")]
    public float TorsoLength { get; set; }

    [JsonPropertyName("slot_top")]
    public string SlotTop { get; set; } = string.Empty;

    [JsonPropertyName("slot_bottom")]
    public string SlotBottom { get; set; } = string.Empty;

    [JsonPropertyName("slot_shoes")]
    public string SlotShoes { get; set; } = string.Empty;

    [JsonPropertyName("slot_gloves")]
    public string SlotGloves { get; set; } = string.Empty;

    [JsonPropertyName("slot_head")]
    public string SlotHead { get; set; } = string.Empty;

    [JsonPropertyName("slot_shoulder")]
    public string SlotShoulder { get; set; } = string.Empty;

    [JsonPropertyName("slot_belt")]
    public string SlotBelt { get; set; } = string.Empty;

    [JsonPropertyName("weapon")]
    public string Weapon { get; set; } = string.Empty;

}

