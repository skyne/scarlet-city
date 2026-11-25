namespace ScarletCity.Protocol;

public static class CharacterCatalog
{
    public static readonly (string Id, string Label)[] BodySets =
    {
        ("male", "Male"),
        ("female", "Female")
    };

    static readonly (string Id, string Label)[] MaleHair =
    {
        ("buzzed", "Buzzed"),
        ("simple_parted", "Side part"),
        ("long", "Long"),
        ("short_street", "Short street")
    };

    static readonly (string Id, string Label)[] FemaleHair =
    {
        ("buzzed_female", "Buzzed"),
        ("buns", "Buns"),
        ("long", "Long"),
        ("short_street", "Short street")
    };

    static readonly (string Id, string Label)[] Outfits =
    {
        ("peasant", "Street"),
        ("ranger", "Field"),
        ("casual", "Casual"),
        ("suit", "Suit"),
        ("worker", "Worker"),
        ("punk", "Punk"),
        ("tactical", "Tactical")
    };

    public static readonly (string Id, string Label)[] Weapons =
    {
        ("none", "None"),
        ("pistol", "Pistol"),
        ("revolver", "Revolver"),
        ("shotgun", "Shotgun"),
        ("smg", "Submachine gun"),
        ("rifle", "Assault rifle"),
        ("sniper", "Sniper rifle"),
        ("bullpup", "Bullpup")
    };

    public static readonly (string Id, string Label)[] HairColors =
    {
        ("#17110e", "Raven"),
        ("#3b2418", "Chestnut"),
        ("#6b3b1f", "Auburn"),
        ("#a4672f", "Copper"),
        ("#d4b06a", "Honey"),
        ("#8d8d8d", "Ash"),
        ("#e6e6e6", "Silver"),
        ("#7a3f52", "Wine")
    };

    public static readonly (string Id, string Label)[] EyeColors =
    {
        ("#4a2f1e", "Dark brown"),
        ("#8a5a2b", "Amber"),
        ("#3f6fa8", "Blue"),
        ("#4f7a52", "Green"),
        ("#7a5aa8", "Violet"),
        ("#b04848", "Red")
    };

    public static readonly (string Id, string Label)[] SkinColors =
    {
        ("#7a4b32", "Deep"),
        ("#a96f4d", "Tan"),
        ("#c38a62", "Warm"),
        ("#d8a07a", "Light"),
        ("#f0c19c", "Fair"),
        ("#9c8f7f", "Ash")
    };

    public static readonly (string Id, string Label)[] OutfitColors =
    {
        ("#ffffff", "Natural"),
        ("#6b3f3f", "Oxblood"),
        ("#3f5a6b", "Slate"),
        ("#4f6b3f", "Moss"),
        ("#6b5a3f", "Sand"),
        ("#3f3f4f", "Charcoal")
    };

    public static readonly (string Key, string Label)[] Sliders =
    {
        ("Height", "Height"),
        ("Head", "Head size"),
        ("Neck", "Neck length"),
        ("Shoulders", "Shoulder width"),
        ("Bust", "Chest / bust"),
        ("Waist", "Waist"),
        ("Hips", "Hip width"),
        ("Bum", "Bum"),
        ("Muscle", "Muscle"),
        ("Arms", "Arm thickness"),
        ("Legs", "Leg thickness"),
        ("LegLength", "Leg length"),
        ("TorsoLength", "Torso length")
    };

    public static readonly (string Key, string Label)[] ClothingSlots =
    {
        ("SlotTop", "Top"),
        ("SlotBottom", "Bottom"),
        ("SlotShoes", "Shoes"),
        ("SlotGloves", "Gloves / arms"),
        ("SlotHead", "Headwear"),
        ("SlotShoulder", "Shoulders"),
        ("SlotBelt", "Belt / waist")
    };

    static readonly (string Id, string Label)[] ClothingSlotOptions =
    {
        ("none", "None"),
        ("peasant", "Street"),
        ("ranger", "Field"),
        ("casual", "Casual"),
        ("suit", "Suit"),
        ("worker", "Worker"),
        ("punk", "Punk"),
        ("tactical", "Tactical")
    };

    static readonly (string Id, string Label)[] AccessorySlotOptions =
    {
        ("none", "None"),
        ("ranger", "Field")
    };

    public static CharacterAppearance Default => new()
    {
        BodySet = "male",
        Hair = "buzzed",
        Outfit = "peasant",
        Beard = false,
        Brows = true,
        HairColor = "#2a1b14",
        BeardColor = "#2a1b14",
        EyeColor = "#6b4a2f",
        SkinColor = "#d8a07a",
        OutfitColor = "#ffffff",
            Height = 0.5f,
            Head = 0.5f,
            Neck = 0.5f,
            Shoulders = 0.5f,
            Bust = 0.35f,
            Waist = 0.5f,
            Hips = 0.5f,
            Bum = 0.35f,
            Muscle = 0.35f,
            Arms = 0.5f,
            Legs = 0.5f,
            LegLength = 0.5f,
            TorsoLength = 0.5f,
            SlotTop = "peasant",
            SlotBottom = "peasant",
            SlotShoes = "peasant",
            SlotGloves = "peasant",
            SlotHead = "none",
            SlotShoulder = "none",
            SlotBelt = "none",
            Weapon = "pistol"
        };

    public static IReadOnlyList<(string Id, string Label)> HairOptions(string bodySet) =>
        bodySet == "female" ? FemaleHair : MaleHair;

    public static IReadOnlyList<(string Id, string Label)> OutfitOptions(string bodySet) => Outfits;

    public static IReadOnlyList<(string Id, string Label)> SlotOptions(string slotKey) =>
        slotKey is "SlotHead" or "SlotShoulder" or "SlotBelt" ? AccessorySlotOptions : ClothingSlotOptions;

    public static bool CanGrowBeard(string bodySet) => bodySet != "female";

    public static CharacterAppearance Copy(this CharacterAppearance? appearance)
    {
        var a = appearance.Normalized();
        return a;
    }

    public static CharacterAppearance Normalized(this CharacterAppearance? appearance)
    {
        var body = BodySets.Any(b => b.Id == appearance?.BodySet) ? appearance!.BodySet : BodySets[0].Id;
        var hairs = HairOptions(body);
        var outfits = OutfitOptions(body);
        var hair = hairs.Any(h => h.Id == appearance?.Hair) ? appearance!.Hair : hairs[0].Id;
        if (appearance?.Hair == "beard")
        {
            hair = hairs[0].Id;
        }
        var beard = CanGrowBeard(body) && ((appearance?.Beard ?? false) || appearance?.Hair == "beard");
        var outfit = outfits.Any(o => o.Id == appearance?.Outfit) ? appearance!.Outfit : outfits[0].Id;
        string slot(string key, string? v, string def)
        {
            var allowed = SlotOptions(key).Select(s => s.Id).ToArray();
            return !string.IsNullOrEmpty(v) && allowed.Contains(v) ? v! : def;
        }
        return new CharacterAppearance
        {
            BodySet = body,
            Hair = hair,
            Outfit = outfit,
            Beard = beard,
            Brows = appearance?.Brows ?? true,
            HairColor = Color(appearance?.HairColor, "#2a1b14"),
            BeardColor = Color(appearance?.BeardColor, appearance?.HairColor ?? "#2a1b14"),
            EyeColor = Color(appearance?.EyeColor, "#6b4a2f"),
            SkinColor = Color(appearance?.SkinColor, "#d8a07a"),
            OutfitColor = Color(appearance?.OutfitColor, "#ffffff"),
            Height = Clamp(appearance?.Height ?? 0.5f, 0.5f),
            Head = Clamp(appearance?.Head ?? 0.5f, 0.5f),
            Neck = Clamp(appearance?.Neck ?? 0.5f, 0.5f),
            Shoulders = Clamp(appearance?.Shoulders ?? 0.5f, 0.5f),
            Bust = Clamp(appearance?.Bust ?? (body == "female" ? 0.5f : 0.35f), 0.5f),
            Waist = Clamp(appearance?.Waist ?? 0.5f, 0.5f),
            Hips = Clamp(appearance?.Hips ?? 0.5f, 0.5f),
            Bum = Clamp(appearance?.Bum ?? (body == "female" ? 0.45f : 0.35f), 0.5f),
            Muscle = Clamp(appearance?.Muscle ?? 0.35f, 0.5f),
            Arms = Clamp(appearance?.Arms ?? 0.5f, 0.5f),
            Legs = Clamp(appearance?.Legs ?? 0.5f, 0.5f),
            LegLength = Clamp(appearance?.LegLength ?? 0.5f, 0.5f),
            TorsoLength = Clamp(appearance?.TorsoLength ?? 0.5f, 0.5f),
            SlotTop = slot("SlotTop", appearance?.SlotTop, "peasant"),
            SlotBottom = slot("SlotBottom", appearance?.SlotBottom, "peasant"),
            SlotShoes = slot("SlotShoes", appearance?.SlotShoes, "peasant"),
            SlotGloves = slot("SlotGloves", appearance?.SlotGloves, "peasant"),
            SlotHead = slot("SlotHead", appearance?.SlotHead, "none"),
            SlotShoulder = slot("SlotShoulder", appearance?.SlotShoulder, "none"),
            SlotBelt = slot("SlotBelt", appearance?.SlotBelt, "none"),
            Weapon = Weapons.Any(w => w.Id == appearance?.Weapon) ? appearance!.Weapon : Weapons[1].Id
        };
    }

    public static float GetSlider(this CharacterAppearance? appearance, string key)
    {
        var a = appearance.Normalized();
        return key switch
        {
            "Height" => a.Height,
            "Head" => a.Head,
            "Neck" => a.Neck,
            "Shoulders" => a.Shoulders,
            "Bust" => a.Bust,
            "Waist" => a.Waist,
            "Hips" => a.Hips,
            "Bum" => a.Bum,
            "Muscle" => a.Muscle,
            "Arms" => a.Arms,
            "Legs" => a.Legs,
            "LegLength" => a.LegLength,
            "TorsoLength" => a.TorsoLength,
            _ => 0.5f
        };
    }

    public static void SetSlider(this CharacterAppearance appearance, string key, float value)
    {
        var v = Math.Clamp(value, 0f, 1f);
        switch (key)
        {
            case "Height": appearance.Height = v; break;
            case "Head": appearance.Head = v; break;
            case "Neck": appearance.Neck = v; break;
            case "Shoulders": appearance.Shoulders = v; break;
            case "Bust": appearance.Bust = v; break;
            case "Waist": appearance.Waist = v; break;
            case "Hips": appearance.Hips = v; break;
            case "Bum": appearance.Bum = v; break;
            case "Muscle": appearance.Muscle = v; break;
            case "Arms": appearance.Arms = v; break;
            case "Legs": appearance.Legs = v; break;
            case "LegLength": appearance.LegLength = v; break;
            case "TorsoLength": appearance.TorsoLength = v; break;
        }
    }

    public static string GetSlot(this CharacterAppearance? appearance, string key)
    {
        var a = appearance.Normalized();
        return key switch
        {
            "SlotTop" => a.SlotTop,
            "SlotBottom" => a.SlotBottom,
            "SlotShoes" => a.SlotShoes,
            "SlotGloves" => a.SlotGloves,
            "SlotHead" => a.SlotHead,
            "SlotShoulder" => a.SlotShoulder,
            "SlotBelt" => a.SlotBelt,
            _ => "none"
        };
    }

    public static void SetSlot(this CharacterAppearance appearance, string key, string value)
    {
        var allowed = SlotOptions(key).Select(s => s.Id).ToArray();
        var v = allowed.Contains(value) ? value : "none";
        switch (key)
        {
            case "SlotTop": appearance.SlotTop = v; break;
            case "SlotBottom": appearance.SlotBottom = v; break;
            case "SlotShoes": appearance.SlotShoes = v; break;
            case "SlotGloves": appearance.SlotGloves = v; break;
            case "SlotHead": appearance.SlotHead = v; break;
            case "SlotShoulder": appearance.SlotShoulder = v; break;
            case "SlotBelt": appearance.SlotBelt = v; break;
        }
    }

    public static int ToVariant(this CharacterAppearance? appearance)
    {
        var a = appearance.Normalized();
        var body = Math.Max(0, Array.FindIndex(BodySets, b => b.Id == a.BodySet));
        var hair = Math.Max(0, Array.FindIndex(HairOptions(a.BodySet).ToArray(), h => h.Id == a.Hair));
        var outfit = Math.Max(0, Array.FindIndex(OutfitOptions(a.BodySet).ToArray(), o => o.Id == a.Outfit));
        return body | (hair << 4) | (outfit << 8) | (a.Beard ? 1 << 12 : 0);
    }

    static float Clamp(float value, float fallback) =>
        float.IsFinite(value) ? Math.Clamp(value, 0f, 1f) : fallback;

    static string Color(string? value, string fallback) =>
        value?.Length == 7 && value[0] == '#' && value.Skip(1).All(Uri.IsHexDigit)
            ? value
            : fallback;
}
