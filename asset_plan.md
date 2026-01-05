# BLACKWING Asset Integration Plan

This document outlines opportunities for graphical asset integration in the BLACKWING frontend. Currently, the UI uses zero image assets—everything is Tailwind CSS utilities, inline SVG shapes, and CSS-generated patterns.

## Current Visual State

| Element | Current Implementation |
|---------|----------------------|
| Ships | Simple triangle SVG polygons |
| Locations | Colored circles via Tailwind |
| Icons | Inline SVG strokes (close X, etc.) |
| Backgrounds | CSS radial gradients (stars) |
| Branding | Text "BLACKWING" in amber |

---

## High-Impact Asset Opportunities

### 1. Ship Visuals

**Location:** `crates/bw-frontend/src/components/game/sector_map.rs`

**Current:** All ships rendered as identical triangles:
```rust
<polygon points="8,0 16,16 8,12 0,16" />
```

**Available Data for Variants:**
- `ship_class`: "Corvette", "Frigate", "Cruiser", etc.
- `faction_tag`: Player's faction affiliation
- `hull_percent`: 0-100 damage state
- `is_hostile`: Friend/foe distinction
- `is_player`: Highlight player's own ship

**Asset Opportunities:**
- Ship class silhouettes/sprites
- Faction color overlays or livery variants
- Damage state variations (smoke, sparks, hull breaches)
- Engine glow animations
- Shield bubble effect when shields > 0

**Suggested Assets:**
```
/ships/
  corvette.svg
  corvette_damaged.svg (hull < 50%)
  corvette_critical.svg (hull < 25%)
  frigate.svg
  frigate_damaged.svg
  frigate_critical.svg
  cruiser.svg
  ...
```

---

### 2. Location Markers

**Location:** `crates/bw-frontend/src/components/game/sector_map.rs:231-238`

**Current:** Colored circles differentiated only by Tailwind classes:
```rust
"station" => ("bg-blue-500", "w-4 h-4"),
"mining" => ("bg-amber-500", "w-3 h-3"),
"jumpgate" => ("bg-purple-500", "w-4 h-4"),
"debris" => ("bg-slate-500", "w-3 h-3"),
"anomaly" => ("bg-cyan-500", "w-3 h-3"),
```

**Available Data for Variants:**
- `location_type`: station, mining, jumpgate, debris, anomaly
- `faction_tag`: Which faction controls it
- `services`: What's available (for stations)

**Asset Opportunities:**
- Distinct icons per location type
- Faction-branded station variants
- Animated jumpgate (swirling portal effect)
- Pulsing anomaly effect
- Debris field scatter pattern

**Suggested Assets:**
```
/locations/
  station.svg
  station_hostile.svg
  mining_outpost.svg
  jumpgate.svg (or animated jumpgate.gif/webp)
  debris_field.svg
  anomaly.svg
  anomaly_pulse.svg (animation frames)
```

---

### 3. Sector Map Background

**Location:** `crates/bw-frontend/style/main.css:17-68`

**Current:** CSS-generated star pattern with twinkle animation:
```css
.stars-pattern::before {
    background-image: radial-gradient(2px 2px at 20px 30px, ...);
}
```

**Available Data for Variants:**
- `sector_name`: Could map to different environments
- `danger_level`: Safe, Moderate, High, Extreme

**Asset Opportunities:**
- Sector-specific backgrounds (nebulae, asteroid belts, deep space)
- Danger-level atmospheric overlays (red tint for dangerous sectors)
- Parallax star layers for depth
- Animated environmental effects

**Suggested Assets:**
```
/backgrounds/
  starfield_default.png
  nebula_blue.png
  nebula_red.png
  asteroid_field.png
  danger_overlay_moderate.png (semi-transparent)
  danger_overlay_high.png
  danger_overlay_extreme.png
```

---

### 4. Station Services Panel

**Location:** `crates/bw-frontend/src/components/game/station_panel.rs:43-70`

**Current:** Text-only buttons:
- Refuel - "Restore fuel to 100%"
- Rearm - "Restore ammunition to 100%"
- Repair - "Restore hull integrity"
- Shore Leave - "Restore crew morale"

**Asset Opportunities:**
- Service icons for each button
- Station interior background/frame when docked
- Animated service effects (fuel flowing, sparks for repair)

**Suggested Assets:**
```
/services/
  refuel.svg (fuel pump/canister)
  rearm.svg (missile/ammunition)
  repair.svg (wrench/welding)
  shore_leave.svg (crew/recreation)

/ui/
  station_panel_frame.svg
  station_interior_bg.png
```

---

### 5. Mission Cards

**Location:** `crates/bw-frontend/src/components/game/mission_panel.rs:100-107`

**Current:** Text labels with color coding:
```rust
"PirateIntercept" => "text-red-400",
"DistressSignal" => "text-yellow-400",
"AsteroidThreat" => "text-orange-400",
"TerroristPlot" => "text-purple-400",
"Investigation" => "text-blue-400",
```

**Asset Opportunities:**
- Mission type icons
- High-profile mission badge/frame
- Mission card backgrounds by type
- Progress indicator graphics

**Suggested Assets:**
```
/missions/
  pirate_intercept.svg (skull/crossbones)
  distress_signal.svg (SOS beacon)
  asteroid_threat.svg (asteroid)
  terrorist_plot.svg (bomb/conspiracy)
  investigation.svg (magnifying glass)
  high_profile_badge.svg

  card_bg_combat.png
  card_bg_rescue.png
  card_bg_investigation.png
```

---

### 6. Ship Status Panel

**Location:** `crates/bw-frontend/src/components/game/ship_status.rs`

**Current:** Progress bars and text grids for:
- Hull/Shields (progress bars)
- Systems: Engines, Weapons, Sensors, Comms (text status)
- Weapons: Railgun, Point Defense (text status)

**Asset Opportunities:**
- Ship silhouette diagram with damage highlighting
- System status icons (green/amber/red variants)
- Weapon icons
- Animated shield effect

**Suggested Assets:**
```
/ship_status/
  ship_silhouette.svg (top-down or side view)
  ship_silhouette_overlay_damage.svg

  system_engines.svg
  system_weapons.svg
  system_sensors.svg
  system_comms.svg

  weapon_railgun.svg
  weapon_point_defense.svg
```

---

### 7. Faction Identity

**Location:** `crates/bw-frontend/src/pages/register.rs` (faction selection)

**Current:** Small colored dot + text name:
```rust
<span class="w-3 h-3 rounded-full" style="background-color: {color}" />
<span class="font-semibold">{name}</span>
```

**Available Factions:** Loaded dynamically from API with:
- `name`, `tag`, `description`, `philosophy`, `color`

**Asset Opportunities:**
- Faction emblems/crests
- Faction selection card backgrounds
- Faction-themed UI accents throughout game
- Ship livery system

**Suggested Assets:**
```
/factions/
  {tag}_emblem.svg (for each faction)
  {tag}_emblem_small.svg (16x16 for inline use)
  {tag}_banner.png (card background)
  {tag}_pattern.svg (repeating pattern for UI accents)
```

---

### 8. Combat & Effects

**Location:** `crates/bw-frontend/src/components/game/combat_log.rs`

**Current:** Text log with color-coded messages

**Asset Opportunities:**
- Weapon fire effects on map
- Explosion animations
- Shield impact flashes
- Combat event icons in log

**Suggested Assets:**
```
/effects/
  railgun_fire.gif (or sprite sheet)
  point_defense_fire.gif
  explosion_small.gif
  explosion_large.gif
  shield_impact.gif

/combat/
  icon_hit.svg
  icon_miss.svg
  icon_critical.svg
  icon_destroyed.svg
```

---

### 9. Branding & UI Chrome

**Location:** `crates/bw-frontend/src/pages/game.rs:144`

**Current:** Text "BLACKWING" in Tailwind amber

**Asset Opportunities:**
- Logo/wordmark
- Animated logo for loading screen
- UI panel frames/borders
- Button styles
- Custom cursor

**Suggested Assets:**
```
/branding/
  logo.svg
  logo_animated.gif (loading screen)
  wordmark.svg

/ui/
  panel_frame.svg (9-slice or border-image)
  button_primary.svg
  button_secondary.svg
  cursor_default.png
  cursor_target.png
```

---

## Technical Integration Notes

### Asset Serving

Currently no static asset pipeline exists. Options:

1. **Trunk (current build tool)** - Add to `index.html`:
   ```html
   <link data-trunk rel="copy-dir" href="assets" />
   ```

2. **Server-side** - Serve from `bw-server` static files

### Recommended Formats

| Use Case | Format | Reasoning |
|----------|--------|-----------|
| Icons, UI elements | SVG | Scalable, small size, CSS-styleable |
| Ship sprites | SVG or PNG | SVG if simple, PNG if detailed |
| Backgrounds | WebP/PNG | Compressed raster for complex scenes |
| Animations | WebP/GIF/Sprite sheets | WebP preferred for quality/size |

### CSS Integration

For SVG assets, can inline or reference:
```rust
// Inline (current approach, good for small icons)
<svg>...</svg>

// External reference (better for larger/reusable assets)
<img src="/assets/ships/corvette.svg" />

// Background image
style="background-image: url('/assets/backgrounds/nebula.png')"
```

### Sprite Sheet Consideration

For many ship variants, consider a sprite sheet approach:
```css
.ship-corvette { background-position: 0 0; }
.ship-frigate { background-position: -32px 0; }
.ship-cruiser { background-position: -64px 0; }
```

---

## Priority Ranking

| Priority | Asset Category | Impact | Effort |
|----------|---------------|--------|--------|
| 1 | Ship sprites | High - core gameplay visibility | Medium |
| 2 | Location icons | High - map readability | Low |
| 3 | Logo/branding | High - first impression | Low |
| 4 | Faction emblems | Medium - identity/immersion | Medium |
| 5 | Mission icons | Medium - UI clarity | Low |
| 6 | Sector backgrounds | Medium - atmosphere | Medium |
| 7 | Service icons | Low - small UI area | Low |
| 8 | Combat effects | Low - polish | High |
| 9 | Ship status diagram | Low - nice-to-have | Medium |

---

## Appendix: Complete Asset Manifest

```
/assets/
  /branding/
    logo.svg
    logo_loading.gif
    wordmark.svg

  /ships/
    corvette.svg
    corvette_damaged.svg
    corvette_critical.svg
    frigate.svg
    frigate_damaged.svg
    frigate_critical.svg
    cruiser.svg
    ...

  /locations/
    station.svg
    station_hostile.svg
    mining.svg
    jumpgate.svg
    jumpgate_active.gif
    debris.svg
    anomaly.svg

  /factions/
    {tag}_emblem.svg
    {tag}_emblem_small.svg
    {tag}_banner.png

  /missions/
    pirate_intercept.svg
    distress_signal.svg
    asteroid_threat.svg
    terrorist_plot.svg
    investigation.svg
    high_profile_badge.svg

  /services/
    refuel.svg
    rearm.svg
    repair.svg
    shore_leave.svg

  /ship_status/
    silhouette.svg
    system_engines.svg
    system_weapons.svg
    system_sensors.svg
    system_comms.svg
    weapon_railgun.svg
    weapon_point_defense.svg

  /backgrounds/
    starfield.png
    nebula_blue.png
    nebula_red.png
    asteroid_field.png

  /effects/
    railgun_fire.gif
    explosion.gif
    shield_impact.gif

  /ui/
    panel_frame.svg
    cursor_default.png
    cursor_target.png
```
