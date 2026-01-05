# Game VM Transformation Plan

Transform Blackwing from a fixed space-game implementation into a schema-driven Game VM capable of running any game type.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Game Definition Layer                     │
│  (TOML schemas + Rhai scripts defining entities/events)     │
├─────────────────────────────────────────────────────────────┤
│                      Plugin System                           │
│         (Load game modules, register schemas/bindings)       │
├─────────────────────────────────────────────────────────────┤
│                     VM Core Layer                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │ Generic  │  │ Dynamic  │  │ Schema   │  │ Generic  │    │
│  │ Entities │  │ Events   │  │ Validator│  │ State    │    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘    │
├─────────────────────────────────────────────────────────────┤
│                   Preserved Infrastructure                   │
│    (Overlay/transactions, coroutines, permissions, Rhai)    │
└─────────────────────────────────────────────────────────────┘
```

## New Crate Structure

```
crates/
  vm-core/           # Generic entity/component system, state, events
  vm-schema/         # Schema definitions, TOML loader, validation
  vm-scripting/      # Rhai engine + generic bindings
  vm-runtime/        # Server runtime, game loop, persistence
  vm-shared/         # Client-server DTOs

  # Preserved (minimal changes)
  bw-ai/             # Already generic
  bw-auth/           # Game-agnostic
  bw-persistence/    # Adapt to generic entities
```

## Gaps Identified (via Test Cases)

### Test Case 1: Dating Sim
| Requirement | Solution |
|-------------|----------|
| Custom enums (RelationshipStage) | Schema-defined enum types |
| Computed properties (stage from affection) | Schema-defined derivations |
| Game-specific script APIs | Script-defined functions that wrap primitives |
| Time/scheduling | Optional VM module, not hardcoded |
| Domain mutations (add_affection) | Script-defined mutation helpers |

### Test Case 2: Final Fantasy 1 (1987)
| Requirement | Solution |
|-------------|----------|
| Turn-based combat | **Combat Module**: Action queues, initiative, turn resolution |
| Party system | Entity groups with shared state, party-level computed stats |
| Multi-scale worlds | **Spatial Module**: World map vs dungeon scale, transitions |
| Random encounters | Event triggers on movement, weighted enemy pools |
| Stats/leveling | Computed properties with level-based formulas |
| Equipment effects | Component stacking, stat modifiers |

### Test Case 3: Legend of Zelda (1986)
| Requirement | Solution |
|-------------|----------|
| Room-based world | **Spatial Module**: Discrete rooms, screen transitions |
| Persistent room state | Per-room entity tracking (defeated enemies, opened chests) |
| Real-time action | Frame-based tick system, hitbox components |
| Item-gated areas | Unlock conditions referencing inventory state |
| Puzzles | Trigger conditions (all enemies dead, block on switch) |

### Test Case 4: Holdsmith (Narrative)
| Requirement | Solution |
|-------------|----------|
| Scene selection | **Narrative Module**: Weighted pools, tags, cooldowns |
| Choice branching | State transform syntax (`fuel -= 4`) |
| Chronicle/history | Append-only event log, queryable |
| Conditional choices | Requirements expressions evaluated against state |
| Context-aware scenes | Scene metadata filtering (journey vs port) |

---

## Core Type Changes

### 1. Dynamic Entity System (replaces Ship, Player, etc.)

```rust
// vm-core/src/entity.rs
pub struct EntityTypeId(pub String);      // "ship", "building", "card"
pub struct ComponentTypeId(pub String);   // "health", "position", "inventory"

pub struct Entity {
    pub id: Uuid,
    pub type_id: EntityTypeId,
    pub components: HashMap<ComponentTypeId, ComponentData>,
}

pub struct ComponentData {
    pub fields: rhai::Map,  // Schema-validated dynamic data
}
```

### 2. Schema-Driven Definitions (Enhanced)

```toml
# games/blackwing/schema/entities.toml
[entity_types.ship]
name = "Ship"
required_components = ["transform", "health", "ship_stats"]
optional_components = ["cargo", "upgrades", "ai_controller"]
behaviors = ["behaviors/ship_base.rhai"]

[component_types.health]
name = "Health"
track_changes = true
[component_types.health.fields]
current = { type = "float", required = true, min = 0.0 }
max = { type = "float", required = true }

[event_types.entity_damaged]
name = "EntityDamaged"
[event_types.entity_damaged.fields]
target_id = { type = "uuid", required = true }
damage = { type = "float", required = true }
```

**NEW: Enum Types in Schema** (supports dating sim's RelationshipStage):

```toml
# games/dating-sim/schema/types.toml
[enum_types.relationship_stage]
name = "RelationshipStage"
variants = ["Stranger", "Acquaintance", "Friend", "CloseFriend", "Romantic", "Partner"]

[enum_types.location_type]
name = "LocationType"
variants = ["Farm", "Commune", "Market", "Residence", "PublicSpace"]
```

**NEW: Computed/Derived Properties**:

```toml
# games/dating-sim/schema/components.toml
[component_types.relationship]
name = "Relationship"
[component_types.relationship.fields]
player_id = { type = "uuid", required = true }
character_id = { type = "uuid", required = true }
affection = { type = "int", required = true, default = 0 }
stage = { type = "enum:relationship_stage", computed = true }

# Derivation rules (Rhai expressions)
[component_types.relationship.derivations]
stage = """
  if affection >= 900 { "Partner" }
  else if affection >= 700 { "Romantic" }
  else if affection >= 500 { "CloseFriend" }
  else if affection >= 300 { "Friend" }
  else if affection >= 100 { "Acquaintance" }
  else { "Stranger" }
"""
```

**NEW: Script-Defined API Extensions** (game-specific helpers):

```rhai
// games/dating-sim/scripts/api/affection_api.rhai
// These become available as global functions in all game scripts

fn add_affection(character_id, amount) {
    let rel = query_entities(#{
        entity_type: "relationship",
        filter: `character_id == "${character_id}" && player_id == current_player_id()`
    })[0];

    let current = get_component(rel.id, "relationship").affection;
    set_component(rel.id, "relationship", #{ affection: current + amount });

    // Stage is auto-computed via derivation!
    emit("affection_changed", #{ character_id: character_id, delta: amount });
}

fn get_relationship_stage(character_id) {
    let rel = find_relationship(character_id);
    get_component(rel.id, "relationship").stage  // Computed property
}

fn give_gift(character_id, gift_id) {
    let char = query_entity(character_id);
    let prefs = get_component(character_id, "gift_preferences");
    let multiplier = prefs[gift_id] ?? 1.0;
    add_affection(character_id, 10 * multiplier);
}
```

### 3. Generic State Provider (replaces type-specific accessors)

```rust
// vm-core/src/state/accessor.rs
pub trait StateProvider: Send + Sync {
    fn get_entity(&self, id: Uuid) -> Option<EntitySnapshot>;
    fn query_entities(&self, query: EntityQuery) -> Vec<EntitySnapshot>;
    fn get_component(&self, entity_id: Uuid, component: &ComponentTypeId) -> Option<ComponentData>;
    fn apply_mutations(&self, mutations: Vec<Mutation>) -> Vec<MutationResult>;
}
```

### 4. Dynamic Events (replaces GameEventType enum)

```rust
// vm-core/src/events/mod.rs
pub struct EventTypeId(pub String);  // Dynamic, defined by schema

pub struct GameEvent {
    pub event_type: EventTypeId,  // Not an enum
    pub data: rhai::Map,          // Schema-validated payload
    // ... actor_id, target_id, sector_id preserved
}
```

### 5. Generic Script API

```rhai
// New generic API (works with any game)
let entity = query_entity(id);
let health = get_component(entity_id, "health");
set_component(entity_id, "health", #{ current: health.current - 10 });
spawn_entity("ship", #{ transform: #{ position: [0,0,0] }, health: #{ current: 100, max: 100 } });
emit("entity_damaged", #{ target_id: id, damage: 10 });
```

### 6. Plugin System

```rust
pub trait GamePlugin: Send + Sync {
    fn id(&self) -> &str;
    fn register_schemas(&self, registry: &mut SchemaRegistry);
    fn register_bindings(&self, engine: &mut Engine);
    fn initialize(&self, ctx: &mut GameContext) -> Result<(), PluginError>;
    fn tick(&self, ctx: &mut GameContext, delta: f64);
}
```

### 7. Optional VM Modules

Games opt-in to built-in modules via plugin manifest. Each module provides components, events, and script APIs.

```toml
# games/ff1-clone/plugin.toml
[plugin]
id = "ff1-clone"

[modules]
spatial = { mode = "discrete", scale = "room" }  # Room-based
combat = { mode = "turn_based" }
party = { enabled = true }
inventory = { enabled = true }

[api_extensions]
paths = ["scripts/api/"]
```

---

#### **Time Module** (Dating Sim, Stardew-likes)
```toml
[modules.time]
enabled = true
tick_rate = "1h"  # Game time per real tick
```
Provides:
- `get_game_time()` → `{ day, hour, weekday }`
- `advance_time(hours)`, `is_time_between(start, end)`
- `schedule` component for entities
- `time_changed` event

---

#### **Spatial Module** (Zelda, FF1, Metroidvanias)
```toml
[modules.spatial]
mode = "discrete"    # or "continuous" for physics-based
scale = "room"       # or "tile", "world_map"
transitions = true   # Room/screen transitions
persistence = true   # Remember room state
```
Provides:
- `position` component (room_id + local coords)
- `room` entity type with exits, spawn points
- `enter_room(room_id)`, `get_adjacent_rooms()`
- `room_entered`, `room_exited` events
- Per-room entity persistence (track defeated enemies, opened chests)

---

#### **Combat Module** (FF1, Zelda, Blackwing)
```toml
[modules.combat]
mode = "turn_based"  # or "real_time", "action"
```
**Turn-based** provides:
- `combat_state` entity (participants, turn order, phase)
- `queue_action(entity_id, action)`, `resolve_turn()`
- `initiative` component, `turn_started` event
- Action types: attack, defend, item, flee, spell

**Real-time/Action** provides:
- `hitbox` component (shape, layers, damage)
- `check_collision(a, b)`, `entities_in_range(pos, radius)`
- `collision` event with damage/knockback
- I-frames, stun states

---

#### **Party Module** (FF1, Pokemon, dating sim wingman)
```toml
[modules.party]
enabled = true
max_size = 4
```
Provides:
- `party` entity type grouping multiple characters
- `party_member` component (slot, active/reserve)
- `add_to_party(entity_id)`, `remove_from_party()`
- Party-level computed stats (total HP, average level)
- `party_changed` event

---

#### **Inventory Module** (Most games)
```toml
[modules.inventory]
enabled = true
stacking = true
weight_limit = false
```
Provides:
- `inventory` component (slots, items)
- `item` entity type with `usable`, `equippable`, `stackable` components
- `add_item()`, `remove_item()`, `use_item()`, `equip_item()`
- Equipment slots with stat modifiers
- `item_acquired`, `item_used` events

---

#### **Narrative Module** (Holdsmith, visual novels, RPG dialogue)
```toml
[modules.narrative]
enabled = true
chronicle = true     # Track choice history
```
Provides:
- `scene` entity type with metadata (tags, context, weight, cooldown)
- `choice` component with requirements and state transforms
- `select_scene(context, tags)` - weighted random with cooldowns
- `present_choices(scene_id)`, `make_choice(choice_id)`
- State transform syntax: `{ fuel: "-=4", chronicle: "+='saved_merchant'" }`
- `choice_made`, `scene_completed` events
- Chronicle queries: `has_chronicle_entry("saved_merchant")`

**Scene definition format:**
```toml
# scenes/distress_call.toml
[scene]
id = "distress_call"
context = "journey"
tags = ["space", "moral_choice"]
weight = 1.0
cooldown = 5  # scenes before can repeat

[scene.intro]
text = "Your sensors detect a faint distress signal..."

[scene.choices.respond]
text = "Investigate the signal"
requirements = { fuel = ">= 10" }
transforms = { fuel = "-= 4" }
next = "rescue_attempt"

[scene.choices.ignore]
text = "Continue on your course"
transforms = { chronicle = "+= 'ignored_distress'" }
next = "END"
```

---

#### **Progression Module** (RPGs, roguelikes)
```toml
[modules.progression]
enabled = true
experience_curve = "quadratic"  # or "linear", "custom"
```
Provides:
- `experience` component with level derivation
- `grant_experience(entity_id, amount)`, `check_level_up()`
- Level-up events with stat growth
- Unlock tracking (abilities, areas)

---

#### **Tactics Module** (Advance Wars, Fire Emblem, XCOM)
```toml
[modules.tactics]
enabled = true
grid_size = [15, 10]
fog_of_war = true
```
Provides:
- `grid_position` component (x, y, not room-based)
- `movement_type` component (infantry, treads, air) with terrain costs
- `terrain` entity type with movement/defense modifiers
- `visibility` component, fog of war calculations
- `get_movement_range(entity_id)`, `get_attack_targets()`
- `production` component for factories/bases
- Unit queuing: `queue_unit(factory_id, unit_type)`
- `turn_ended` event triggers AI/enemy phase

---

#### **Command Module** (MUDs, text adventures)
```toml
[modules.command]
enabled = true
parser = "natural"  # or "strict"
```
Provides:
- Command registration: `register_command("go", handle_go)`
- Argument parsing: `go north`, `attack goblin with sword`
- Aliases: `n` → `go north`
- Room descriptions with exit formatting
- `command_received`, `command_failed` events
- Social commands: `say`, `emote`, `tell`
- Help system generation from registered commands

---

### Module Usage by Game Type

| Game Type | Modules Used |
|-----------|--------------|
| **Blackwing** | spatial(continuous), combat(real_time), inventory |
| **Dating Sim** | time, narrative, party(wingman) |
| **FF1 Clone** | spatial(tile), combat(turn_based), party, inventory, progression |
| **Zelda Clone** | spatial(room), combat(action), inventory |
| **Holdsmith** | narrative, time |
| **Card Game** | combat(turn_based), inventory(deck) |
| **Visual Novel** | narrative, time |
| **Advance Wars** | tactics, combat(turn_based), progression |
| **MUD** | spatial(room), command, narrative, inventory, combat(turn_based) |
| **CRPG** | spatial(tile), combat(turn_based), party, inventory, narrative, progression |

---

## Client/Rendering Architecture

The VM must expose state to renderers without coupling to specific frameworks.

### Design Principles

1. **VM is headless** - No rendering code in VM core
2. **State → View is one-way** - VM produces state, clients render it
3. **Multiple client types** - TUI, web (Leptos), native (macroquad/bevy), MUD telnet
4. **Performance-aware** - Minimize Rhai execution, batch updates

### Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│                     Client Layer                             │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐        │
│  │   TUI   │  │  Leptos │  │Macroquad│  │  Telnet │        │
│  │ (ratatui)│  │  (WASM) │  │ (native)│  │  (MUD)  │        │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘        │
│       └───────────┬┴───────────┴┬───────────┘              │
│                   ▼              ▼                          │
│           ┌──────────────┐  ┌──────────────┐               │
│           │  View Model  │  │ Scene Graph  │               │
│           │  (UI state)  │  │ (2D/3D)      │               │
│           └──────┬───────┘  └──────┬───────┘               │
├──────────────────┼─────────────────┼───────────────────────┤
│                  ▼                 ▼                        │
│           ┌────────────────────────────┐                   │
│           │      Render Protocol       │                   │
│           │  (JSON/MessagePack/Proto)  │                   │
│           └────────────┬───────────────┘                   │
├────────────────────────┼───────────────────────────────────┤
│                        ▼                                    │
│              ┌──────────────────┐                          │
│              │   VM Runtime     │                          │
│              │  (game state)    │                          │
│              └──────────────────┘                          │
└─────────────────────────────────────────────────────────────┘
```

### Render Protocol Options

#### Option A: State Diff Streaming (Recommended for most games)
```rust
// VM sends entity changes over WebSocket
pub enum StateUpdate {
    EntityCreated { id: Uuid, type_id: String, components: Map },
    ComponentChanged { entity_id: Uuid, component: String, changes: Map },
    EntityDestroyed { id: Uuid },
    EventOccurred { event_type: String, data: Map },
}
```
- Client maintains local state mirror
- Efficient for entity-based games
- Works with any renderer

#### Option B: View Model (For UI-heavy games)
```rust
// VM produces declarative UI description
pub struct ViewNode {
    pub kind: String,  // "panel", "text", "button", "list"
    pub props: Map,    // { text: "Hello", style: "bold" }
    pub children: Vec<ViewNode>,
    pub events: Vec<String>,  // ["click", "hover"]
}
```
- Good for menus, dialogue, inventory UIs
- Maps to TUI widgets or web components
- Game defines view in Rhai:
```rhai
fn render_inventory(player_id) {
    view("panel", #{ title: "Inventory" }, [
        for item in get_inventory(player_id) {
            view("item_slot", #{ item: item, on_click: "use_item" })
        }
    ])
}
```

#### Option C: Scene Graph (For 2D/3D games)
```rust
// VM produces spatial scene description
pub struct SceneNode {
    pub entity_id: Option<Uuid>,
    pub transform: Transform2D,  // position, rotation, scale
    pub sprite: Option<SpriteRef>,
    pub children: Vec<SceneNode>,
    pub layer: i32,
}
```
- Renderer traverses graph, draws sprites
- Animation state in components, interpolated client-side
- Works with macroquad, bevy, or web canvas

### Hybrid Approach (Recommended)

Most games need multiple protocols:
```toml
# plugin.toml
[rendering]
protocols = ["state_diff", "view_model"]

[rendering.views]
# UI screens defined in Rhai
inventory = "scripts/views/inventory.rhai"
dialogue = "scripts/views/dialogue.rhai"
combat_hud = "scripts/views/combat_hud.rhai"
```

- **State diffs** for game world (entities, positions, health)
- **View model** for UI overlays (menus, dialogues, HUD)
- Client combines both for final render

### Performance: Rhai in WASM

**Problem**: Rhai interpreted in WASM is slow.

**Mitigations**:

1. **Hot paths in Rust** - Collision, pathfinding, visibility in native code
2. **Batch script execution** - Run all entity behaviors in one Rhai call, not per-entity
3. **Minimize crossings** - Pass bulk data, not individual queries
4. **Async script execution** - Long scripts yield, don't block render
5. **Tiered execution**:
   ```toml
   [performance]
   # Scripts run at different frequencies
   per_frame = []  # Only Rust code
   per_tick = ["ai_update", "combat_tick"]  # 10Hz
   on_event = ["damage_handler", "dialogue"]  # As needed
   ```

**Future**: AOT Rhai compilation
- Compile Rhai → Rust → WASM at build time for known scripts
- Interpret only for hot-reloaded content
- Not in Phase 1, but architecture should allow it

### Client Library Structure

```
crates/
  vm-client/           # Client-side state management
    src/
      state_mirror.rs  # Sync state from server
      view_model.rs    # Declarative UI
      scene_graph.rs   # 2D/3D scenes

  vm-client-tui/       # Ratatui adapter
  vm-client-leptos/    # Leptos/web adapter
  vm-client-macroquad/ # Macroquad adapter
```

Each adapter implements:
```rust
pub trait GameRenderer {
    fn apply_state_update(&mut self, update: StateUpdate);
    fn render_view(&mut self, view: ViewNode);
    fn render_scene(&mut self, scene: SceneNode);
    fn handle_input(&mut self) -> Vec<ClientMessage>;
}
```

---

## Multi-Game Architecture (BYOND-inspired)

### Comparison to BYOND

BYOND (Space Station 13 engine) supports:
- Multiple game instances on one server
- Cross-game communication via topics
- Shared player accounts across games
- Hub system for game discovery

Our VM can go further with **first-class multi-game support**.

### Multi-Context Hosting

```
┌─────────────────────────────────────────────────────────────┐
│                      VM Host                                 │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                  Identity Layer                      │    │
│  │   (shared player accounts, cross-game inventory)     │    │
│  └─────────────────────────────────────────────────────┘    │
│                           │                                  │
│  ┌────────────┬───────────┼───────────┬────────────┐        │
│  │            │           │           │            │        │
│  ▼            ▼           ▼           ▼            ▼        │
│ ┌────┐      ┌────┐      ┌────┐      ┌────┐      ┌────┐     │
│ │Game│◄────►│Game│◄────►│Game│◄────►│Game│◄────►│Game│     │
│ │ A  │      │ B  │      │ C  │      │ D  │      │ E  │     │
│ │    │      │    │      │    │      │    │      │    │     │
│ │Blck│      │Date│      │MUD │      │FF1 │      │Zld │     │
│ │wing│      │Sim │      │    │      │    │      │    │     │
│ └────┘      └────┘      └────┘      └────┘      └────┘     │
│    ▲           ▲           ▲           ▲           ▲        │
│    └───────────┴───────────┴───────────┴───────────┘        │
│                    Inter-Game Bus                           │
└─────────────────────────────────────────────────────────────┘
```

### Core Concepts

#### 1. Game Context Isolation
```rust
pub struct GameContext {
    pub id: GameId,
    pub schema: GameSchema,          // This game's types
    pub state: GameState,            // Isolated state
    pub scripts: ScriptEngine,       // Isolated Rhai engine
    pub connections: Vec<PlayerId>,  // Players in this game
}

pub struct VmHost {
    pub contexts: HashMap<GameId, GameContext>,
    pub identity: IdentityService,
    pub bus: InterGameBus,
}
```

#### 2. Inter-Game Communication
```rust
// Games can send messages to other games
pub struct InterGameMessage {
    pub from_game: GameId,
    pub to_game: GameId,
    pub message_type: String,
    pub payload: Map,
}

// Script API
engine.register_fn("send_to_game", |game_id: String, msg_type: String, payload: Map| {
    inter_game_bus.send(InterGameMessage { ... })
});

engine.register_fn("on_game_message", |msg_type: String, handler: FnPtr| {
    register_inter_game_handler(msg_type, handler)
});
```

#### 3. Cross-Game Entity Transfer (Portals)
```rhai
// In Blackwing: Portal to Dating Sim
fn activate_portal(player_id, target_game, target_location) {
    // Serialize player's portable state
    let portable = #{
        identity: get_player_identity(player_id),
        inventory: get_portable_inventory(player_id),  // Only transferable items
        chronicle: get_chronicle(player_id),           // Story flags
    };

    // Request transfer
    send_to_game(target_game, "transfer_request", #{
        player: portable,
        destination: target_location
    });

    // Remove from current game
    disconnect_player(player_id);
}

// In Dating Sim: Receive transfer
fn on_transfer_request(msg) {
    let player = msg.player;

    // Create player in this game's context
    let new_entity = spawn_entity("player", #{
        identity: player.identity,
        // Map inventory to this game's items
        inventory: translate_inventory(player.inventory),
        location: msg.destination
    });

    // Connect player's client to this game
    reconnect_player(player.identity, new_entity);
}
```

#### 4. Shared Identity Layer
```rust
pub struct PlayerIdentity {
    pub id: Uuid,
    pub display_name: String,
    pub global_inventory: Vec<GlobalItem>,  // Items that persist across games
    pub achievements: Vec<Achievement>,
    pub current_game: Option<GameId>,
}

// Some items are game-specific, others are global
pub enum ItemScope {
    GameLocal,        // Only exists in one game
    Portable,         // Can transfer between games
    Global,           // Always available in all games
}
```

#### 5. Bizarre Cross-Game Interactions

**Scenario: Dating Sim character appears in Blackwing**
```rhai
// Dating Sim exposes NPC as inter-game service
fn register_crossover_npc() {
    register_game_service("ivan_the_farmer", |request| {
        match request.type {
            "get_dialogue" => get_ivan_dialogue(request.context),
            "check_relationship" => get_relationship_stage(request.player_identity),
            "gift" => receive_gift(request.item),
        }
    });
}

// Blackwing script queries Dating Sim
fn encounter_mysterious_farmer(player_id) {
    let player_identity = get_player_identity(player_id);

    // Cross-game query
    let relationship = call_game_service("dating-sim", "ivan_the_farmer", #{
        type: "check_relationship",
        player_identity: player_identity
    });

    if relationship.stage == "Partner" {
        // Ivan recognizes you from the dating sim!
        say("Ivan", "Comrade! What are you doing in space?!");
        add_crew_member(player_id, "ivan_guest");
    } else {
        say("Farmer", "Who are you? This is my space tractor.");
    }
}
```

**Scenario: MUD and FF1 share a dungeon**
```toml
# Shared dungeon definition
[shared_content]
dungeon_of_doom = { games = ["mud-world", "ff1-clone"], sync = "real_time" }
```
```rhai
// When player A (in MUD) defeats monster, player B (in FF1) sees it
fn on_monster_defeated(monster_id) {
    broadcast_to_shared_zone("dungeon_of_doom", "monster_defeated", #{
        monster_id: monster_id,
        position: get_position(monster_id)
    });
}
```

### Configuration

```toml
# vm-host.toml
[host]
max_games = 100
shared_identity = true
inter_game_bus = true

[games.blackwing]
plugin = "games/blackwing"
instances = 3  # Run 3 copies

[games.dating-sim]
plugin = "games/dating-sim"
instances = 1

[crossover]
# Define which games can interact
allowed_pairs = [
    ["blackwing", "dating-sim"],
    ["mud-world", "ff1-clone"]
]

[shared_zones]
dungeon_of_doom = { games = ["mud-world", "ff1-clone"] }
```

### Why This Matters

1. **Single deployment** runs multiple games
2. **Cross-promotion** - Players in Game A discover Game B
3. **Shared universe** - Lore/characters span multiple games
4. **Emergent gameplay** - Players find creative cross-game exploits
5. **Community** - Single identity, unified chat, cross-game friends

---

## BYOND/Dream Maker Inspirations

Key patterns from BYOND that we should incorporate:

### 1. Type Path Hierarchy

BYOND uses `/type/path/inheritance`:
```dm
/mob           // Base mobile entity
/mob/player    // Inherits from /mob
/mob/npc       // Also inherits from /mob
/mob/npc/guard // Inherits from /mob/npc
```

**Our adaptation**: Schema inheritance
```toml
# Component inheritance
[component_types.character]
fields = [...]

[component_types.player]
extends = "character"
fields = [...]  # Adds to parent fields

# Entity type inheritance
[entity_types.mob]
components = ["transform", "health"]

[entity_types.player]
extends = "mob"
components = ["inventory", "player_input"]  # Adds to parent

[entity_types.npc]
extends = "mob"
components = ["ai_controller", "dialogue"]
```

### 2. Verbs (Player Actions)

BYOND's killer feature - actions attached to objects:
```dm
/obj/door
    verb/open()
        if(!locked) opened = TRUE
```

**Our adaptation**: Entity-attached actions
```toml
# In schema
[entity_types.door]
components = ["interactable"]
verbs = ["open", "close", "lock", "unlock"]

[verbs.open]
requires = { actor = "nearby", target_state = { locked = false } }
script = "scripts/verbs/door_open.rhai"
```

```rhai
// scripts/verbs/door_open.rhai
fn execute(actor_id, target_id) {
    if get_component(target_id, "door").locked {
        return #{ success: false, message: "The door is locked." };
    }
    set_component(target_id, "door", #{ open: true });
    emit("door_opened", #{ door_id: target_id, actor_id: actor_id });
    #{ success: true }
}
```

Script API for players:
```rhai
// Get verbs available on nearby entities
get_available_verbs(player_id)  // Returns ["open door", "talk to guard", "pick up sword"]

// Execute a verb
execute_verb(player_id, "open", target_id)
```

### 3. Contents Model (Unified Inventory)

BYOND: Everything is just "things inside other things"
```dm
/mob/player/var/list/contents  // Inventory
/obj/chest/var/list/contents   // Chest contents
/turf/var/list/contents        // Things on this tile
```

**Our adaptation**: Universal `contents` component
```toml
[component_types.container]
fields.contents = { type = "array:uuid", default = [] }
fields.capacity = { type = "int", default = 10 }

# Mobs, chests, rooms all have contents
[entity_types.player]
components = ["container"]  # Inventory is just contents

[entity_types.chest]
components = ["container"]

[entity_types.room]
components = ["container"]  # Things in the room
```

```rhai
// Universal operations
put_inside(item_id, container_id)
take_from(item_id, container_id)
get_contents(container_id)

// Works for inventory, chests, rooms, bags...
put_inside(sword_id, player_id)     // Pick up
put_inside(sword_id, chest_id)      // Store in chest
put_inside(player_id, room_id)      // Enter room
```

### 4. Loc (Universal Location)

BYOND: Everything has a `loc` - where it is
```dm
player.loc = room  // Player is in room
sword.loc = player // Sword is on player (inventory)
```

**Our adaptation**: `loc` as universal parent reference
```rust
// Every entity can have a loc
pub struct LocComponent {
    pub parent: Option<Uuid>,  // What contains this entity
}
```

```rhai
get_loc(entity_id)          // What contains this?
set_loc(entity_id, new_loc) // Move to new container

// Location is hierarchical
get_loc(sword_id)    // player_id (in inventory)
get_loc(player_id)   // room_id (in room)
get_loc(room_id)     // area_id (in area)
```

### 5. Built-in Spatial Primitives

BYOND has built-in concepts:
- **Turfs**: Grid tiles (floor, wall, etc.)
- **Areas**: Named regions of turfs
- **Density**: Whether something blocks movement
- **Opacity**: Whether something blocks vision

**Our adaptation**: Spatial module defaults
```toml
[modules.spatial]
mode = "grid"
built_in_components = true  # Enables density, opacity, etc.
```

Provides:
```rust
#[derive(Component)]
pub struct Density {
    pub blocks_movement: bool,
    pub blocks_projectiles: bool,
}

#[derive(Component)]
pub struct Opacity {
    pub blocks_vision: bool,
    pub blocks_light: bool,
}
```

```rhai
can_move_to(entity_id, target_pos)  // Checks density
can_see(from_entity, to_entity)     // Checks opacity/LOS
get_entities_at(position)            // All entities at grid pos
```

### 6. `usr` and `src` Context

BYOND automatically provides context:
- `usr` = The player who initiated the action
- `src` = The object the proc is running on

**Our adaptation**: Script context object
```rhai
// Every script receives ctx
fn on_interact(ctx, params) {
    ctx.actor_id   // Who did this? (like usr)
    ctx.entity_id  // What entity is this script on? (like src)
    ctx.tick       // Current game tick
    ctx.player_id  // If actor is a player
}
```

### 7. Sleep/Spawn (Coroutines)

BYOND has built-in async:
```dm
/mob/proc/delayed_action()
    sleep(10)  // Wait 10 ticks
    do_thing()

spawn(10)      // Do something in 10 ticks
    do_thing()
```

**Our adaptation**: (Already have coroutines!)
```rhai
fn delayed_attack(ctx) {
    say("Charging up...");
    yield_ticks(30);  // Wait 30 ticks
    deal_damage(ctx.target_id, 50);
}

fn spawn_delayed(ticks, callback) {
    schedule_callback(ticks, callback);
}
```

### 8. Topic (RPC)

BYOND games can receive HTTP-like requests:
```dm
/world/Topic(href, href_list)
    if(href_list["action"] == "status")
        return server_status()
```

**Our adaptation**: Game services (already in multi-game section)
```rhai
register_topic("status", fn() {
    #{
        players_online: count_players(),
        uptime: get_uptime()
    }
});
```

### Summary: BYOND Patterns to Adopt

| BYOND Feature | Our Implementation |
|---------------|-------------------|
| `/type/path` inheritance | Schema `extends` |
| Verbs | Entity-attached actions with scripts |
| Contents | Universal `container` component |
| Loc | Universal `loc` component (parent reference) |
| Turfs/Areas | Spatial module with grid tiles |
| Density/Opacity | Built-in spatial components |
| usr/src | Script context object |
| sleep/spawn | Coroutines (existing) |
| Topic | Game services/RPC |

### What We Do Better Than BYOND

1. **Schema-driven** - No recompile for new entity types
2. **Multi-language clients** - Not locked to BYOND client
3. **Modern Rust performance** - Not interpreted DM
4. **Rhai over DM** - More familiar syntax, better tooling
5. **Cross-game** - First-class multi-game support
6. **Module system** - Only load what you need

---

## Minigame Infrastructure Analysis

Analyzing the Neocadia minigame suite reveals **concrete patterns** that the VM must support efficiently.

### Pattern Extraction from 18 Minigames

| Pattern | Games Using It | VM Module |
|---------|----------------|-----------|
| **Ball Physics** | Void Breaker, Jawbreaker, Spring Loaded | `physics` |
| **Rhythm/Timing** | Synth Racer, Scene Stealer, Reel Deal | `timing` |
| **Grid Match-3** | Candy Cascade, Shell Shocked | `grid` |
| **Tower Defense** | Tick Defense | `tactics` + `combat` |
| **Time Management** | Sugar Rush Kitchen | `time` + custom |
| **Idle/Clicker** | Error Garden | `progression` |
| **Quiz/Trivia** | Reel Trivia | `narrative` (choice) |
| **Scoring Universal** | ALL games | `scoring` (NEW) |
| **Token Economy** | ALL games | `economy` (NEW) |
| **Session/Lives** | ALL games | `session` (NEW) |
| **Leaderboards** | ALL games | `leaderboard` (NEW) |
| **Collections** | Reel Deal fish, Shell Shocked shells | `collection` (NEW) |

### New Modules Needed (From Minigame Analysis)

#### **Scoring Module** (Universal)
Every game needs scoring. Extract common patterns:

```toml
[modules.scoring]
enabled = true
```

```rhai
// Universal scoring API
add_score(points)
add_score_multiplied(base, multiplier)  // Applies combo etc.
get_score()
get_high_score()
reset_score()

// Combo system
increment_combo()
break_combo()
get_combo_multiplier()  // 1x → 2x → 3x → 4x

// Timing windows (Synth Racer, Scene Stealer)
check_timing(input_time, target_time, windows)
// Returns: "perfect" | "great" | "good" | "miss"

// Example windows config
let windows = #{
    perfect: 30,   // ±30ms
    great: 60,
    good: 100
};
```

#### **Economy Module** (Token Systems)
```toml
[modules.economy]
enabled = true
daily_bonus = true
session_caps = true
```

```rhai
// Token conversion
let tokens = convert_to_tokens(score, #{
    base: 15,
    divisor: 100,
    cap: 100
});

// Daily bonuses
apply_daily_multiplier(tokens)  // 1.5x first play
apply_streak_bonus(tokens, days)

// Caps
check_session_cap(game_id, earned)
check_daily_cap(earned)

// Cross-game economy
get_total_tokens()
spend_tokens(amount, reason)
```

#### **Session Module** (Lives, Rounds, Time Limits)
```toml
[modules.session]
enabled = true
```

```rhai
// Lives system
set_lives(count)
lose_life()
gain_life()
get_lives()
is_game_over()  // lives <= 0

// Rounds/Waves
start_wave(number)
end_wave()
get_current_wave()

// Time limits
start_timer(seconds)
get_time_remaining()
pause_timer()
resume_timer()
is_time_up()

// Pause/Resume
pause_game()
resume_game()
is_paused()
```

#### **Leaderboard Module**
```toml
[modules.leaderboard]
enabled = true
scopes = ["personal", "daily", "weekly", "alltime"]
```

```rhai
submit_score(game_id, score)
get_leaderboard(game_id, scope, limit)
get_player_rank(game_id, scope)
get_personal_best(game_id)
```

#### **Collection Module** (Unlockables, Achievements)
```toml
[modules.collection]
enabled = true
```

```rhai
// Collections (Reel Deal fish, etc.)
unlock_item(collection_id, item_id)
is_unlocked(collection_id, item_id)
get_collection_progress(collection_id)  // 15/50 fish

// First-catch bonuses
if !is_unlocked("fish", fish_id) {
    unlock_item("fish", fish_id);
    add_bonus_tokens(50);  // Discovery bonus
}

// Achievements
unlock_achievement(id)
get_achievements()
```

#### **Physics Module** (Breakout, Pinball, Launchers)
```toml
[modules.physics]
enabled = true
engine = "simple_2d"  # or "box2d" for complex
```

```rhai
// Ball physics (Void Breaker, Jawbreaker, Spring Loaded)
spawn_ball(position, velocity)
set_ball_velocity(ball_id, velocity)
get_ball_position(ball_id)

// Paddle
set_paddle_position(x)
get_paddle_width()

// Collision callbacks
on_collision("ball", "brick", fn(ball, brick) {
    destroy_entity(brick);
    add_score(brick.points);
    reflect_ball(ball);
});

// Physics config
set_gravity(#{ x: 0, y: 9.8 })
set_bounce_coefficient(entity_id, 0.9)
```

#### **Grid Module** (Match-3, Memory, Puzzles)
```toml
[modules.grid]
enabled = true
```

```rhai
// Board operations
create_grid(width, height, cell_types)
get_cell(x, y)
set_cell(x, y, value)
swap_cells(x1, y1, x2, y2)

// Match detection
find_matches(min_length)  // Returns list of matches
clear_matches(matches)
apply_gravity()  // Cells fall down
fill_empty()     // New cells from top

// Cascades
while find_matches(3).len() > 0 {
    let matches = find_matches(3);
    add_score(matches.len() * 50 * cascade_multiplier);
    clear_matches(matches);
    apply_gravity();
    fill_empty();
    cascade_multiplier += 0.5;
}
```

### Cross-Game Integration Patterns

#### 1. Unified Token Economy
```rhai
// All games deposit to same wallet
fn end_game(score) {
    let tokens = convert_to_tokens(score, game_config.token_formula);
    tokens = apply_daily_multiplier(tokens);
    tokens = check_session_cap(game_id, tokens);
    add_tokens(tokens);

    // Tokens spendable in ANY game/shop
}
```

#### 2. Cross-Game Events
```rhai
// In Reel Deal (fishing game)
fn on_catch_legendary(fish) {
    // Notify other games
    emit_cross_game("neocadia:rare_catch", #{
        game: "reel_deal",
        item: fish.id,
        player: current_player_id()
    });
}

// In Dating Sim, listening
fn on_cross_game_event(event) {
    if event.type == "neocadia:rare_catch" {
        // NPC comments on your fishing skill
        add_dialogue_option("I heard you caught a legendary fish!");
    }
}
```

#### 3. Meta-Progression
```rhai
// Zone restoration from README
fn calculate_zone_bonus() {
    let restoration = get_zone_restoration("pixel_beach");
    return 1.0 + (restoration * 0.002);  // +0.2% per 1% restoration
}
```

#### 4. Composite Games
```rhai
// A game that combines mechanics
// Example: Fishing + Tower Defense
fn start_fishing_defense() {
    // Use fishing module
    let caught = reel_deal_minigame();

    // Fish become tower defense units
    for fish in caught {
        spawn_turret(fish_to_turret(fish));
    }

    // Run tower defense wave
    tick_defense_wave();
}
```

### Minigame Schema Example (Reel Deal)

```toml
# games/neocadia/minigames/reel_deal.toml
[game]
id = "reel_deal"
name = "Reel Deal"
zone = "pixel_beach"
category = "relaxed"

[modules]
scoring = { enabled = true }
economy = { base_award = 20, cap = 75 }
session = { lives = false, timer = false }
collection = { id = "fish", total = 50 }
time = { enabled = false }  # Relaxed, no time pressure

[custom]
bait_types = ["worm", "grub", "lure", "golden"]
zones = ["shore", "mid", "deep"]

[token_formula]
expression = "sum(fish_values) + bonuses"
cap = 75
first_catch_bypass = 3

[events]
on_catch = "scripts/reel_deal/on_catch.rhai"
on_legendary = "scripts/reel_deal/on_legendary.rhai"
```

### Performance: Shared Hot Paths in Rust

These modules have Rust implementations for performance:

```rust
// vm-modules/src/physics.rs
// Ball-paddle collision, reflection math

// vm-modules/src/grid.rs
// Match-3 detection, cascade resolution

// vm-modules/src/timing.rs
// Sub-millisecond timing windows

// vm-modules/src/scoring.rs
// Combo multipliers, cap calculations
```

Rhai scripts call into these, but the hot loops stay in Rust.

---

## Composable Primitives (Not Game-Shaped Modules)

**Key Insight**: Don't implement "combat module" or "inventory module" - these are game-shaped, not composable. Instead, implement **primitives** that compose naturally.

### The Real Primitives (Rust)

| Primitive | What It Does | Composes Into |
|-----------|--------------|---------------|
| **Entity** | ID + type + components | Everything |
| **Component** | Named bag of fields | Any data on entities |
| **Relation** | Entity A → Entity B with type | Party, inventory, targeting, ownership |
| **Schema** | Structure + validation | Type safety |
| **Event** | Happened + payload | All game logic triggers |
| **Query** | Find entities matching criteria | Targeting, match detection, searches |
| **Mutation** | State change | All modifications |
| **Formula** | Computed from other values | Damage calc, stats, derived properties |
| **Constraint** | Rule that must hold | Validation, prerequisites |
| **State Machine** | Entity transitions through states | Combat phases, game states, AI |
| **Timer** | Trigger after duration | Cooldowns, spawning, schedules |
| **Random** | Weighted selection, distributions | Drops, crits, encounters |

### How Games Emerge From Primitives

#### Pokemon
```rhai
// NOT: use_pokemon_module()
// INSTEAD: Compose primitives

// Creature is just an entity with components
let pikachu = spawn_entity("creature", #{
    stats: #{ hp: 35, attack: 55, speed: 90 },
    types: ["electric"],
    moves: ["thunderbolt", "quick_attack"],
    evolution: #{ into: "raichu", trigger: "thunder_stone" }
});

// Party is a RELATION (player OWNS creatures, ordered)
add_relation(player_id, pikachu, "party_member", #{ slot: 0 });

// Battle is a STATE MACHINE
let battle = create_state_machine("battle", #{
    initial: "select_action",
    states: ["select_action", "execute_moves", "check_faint", "next_turn", "end"],
    transitions: [
        #{ from: "select_action", to: "execute_moves", event: "actions_chosen" },
        #{ from: "execute_moves", to: "check_faint", event: "moves_resolved" },
        #{ from: "check_faint", to: "next_turn", event: "no_faint" },
        #{ from: "check_faint", to: "end", event: "party_wiped" },
    ]
});

// Type effectiveness is a RELATION TABLE + FORMULA
let effectiveness = query_relation("type_effectiveness", #{
    attacker_type: "electric",
    defender_type: "water"
}).multiplier;  // Returns 2.0

// Damage is a FORMULA
fn calculate_damage(attacker, defender, move) {
    let base = formula("((2 * level / 5 + 2) * power * attack / defense) / 50 + 2");
    let type_mod = query_relation("type_effectiveness", ...).multiplier;
    let random = random_range(0.85, 1.0);
    return floor(base * type_mod * random);
}

// Evolution is a CONSTRAINT + EVENT_TRIGGER
register_trigger("item_used", fn(ctx) {
    if ctx.item == "thunder_stone" {
        let creature = get_entity(ctx.target_id);
        if creature.evolution.trigger == "thunder_stone" {
            transform_entity(ctx.target_id, creature.evolution.into);
            emit("evolved", #{ from: creature.type, to: creature.evolution.into });
        }
    }
});

// Catching is FORMULA + RANDOM + CONSTRAINT
fn attempt_catch(pokeball, target) {
    let catch_rate = formula("(3 * max_hp - 2 * current_hp) * rate * ball_bonus / (3 * max_hp)");
    let shake_prob = formula("65536 / (255 / catch_rate) ^ 0.1875");
    for i in 0..4 {
        if random() > shake_prob { return false; }  // Broke free
    }
    return true;  // Caught!
}
```

#### Match-3 (Candy Cascade)
```rhai
// Board is COMPONENT with grid layout
let board = spawn_entity("board", #{
    grid: create_grid(8, 8, random_candy_types)
});

// Match detection is a QUERY
fn find_matches() {
    query_grid(board, #{
        pattern: "horizontal_or_vertical",
        min_length: 3,
        same_value: true
    })
}

// Cascade is EVENT_TRIGGER chain
register_trigger("cells_cleared", fn(ctx) {
    apply_gravity(board);       // Cells fall down
    fill_empty(board);          // New cells from top
    let new_matches = find_matches();
    if new_matches.len() > 0 {
        clear_cells(new_matches);
        // Triggers "cells_cleared" again → cascade!
    }
});
```

#### Tower Defense
```rhai
// Path is ORDERED RELATION (waypoints)
for i in 0..waypoints.len() - 1 {
    add_relation(waypoints[i], waypoints[i+1], "path_next");
}

// Enemy movement is TIMER + follow_relation
fn spawn_enemy(type) {
    let enemy = spawn_entity(type, #{ position: waypoints[0] });
    start_timer(enemy, "move_tick", 0.1, fn(ctx) {
        let next = query_relation(ctx.entity_id, "path_next")[0];
        if next == null {
            // Reached end!
            damage_base();
            destroy_entity(ctx.entity_id);
        } else {
            move_toward(ctx.entity_id, next.position, enemy.speed);
        }
    });
}

// Turret targeting is QUERY + FORMULA (priority)
fn find_target(turret) {
    query_entities(#{
        with_component: "enemy",
        in_range: #{ center: turret.position, radius: turret.range }
    })
    .sort_by(|e| formula("distance_to_exit(e)"))  // Target closest to exit
    .first()
}
```

### Primitive API Design

#### Relations (The Unifying Pattern)

Relations are **the key primitive** - they model:
- Inventory (player CONTAINS item)
- Party (player OWNS creature, slot=0)
- Equipment (character EQUIPS sword, slot=weapon)
- Targeting (turret TARGETS enemy)
- Spatial (entity IN room)
- Parent-child (item IN chest IN room)
- Type effectiveness (fire STRONG_AGAINST grass)

```rust
// Rust primitive
pub struct Relation {
    pub from: Uuid,
    pub to: Uuid,
    pub relation_type: String,  // "contains", "owns", "targets", "in", etc.
    pub data: Map,              // { slot: 0, equipped: true, multiplier: 2.0 }
    pub ordered: bool,          // For ordered collections (party slots)
}
```

```rhai
// Rhai API
add_relation(from, to, type, data)
remove_relation(from, to, type)
query_relations(from, type)            // All of type from entity
query_relations_to(to, type)           // All pointing TO entity
get_relation(from, to, type)           // Single relation
has_relation(from, to, type)           // Boolean

// Common patterns built on relations
fn get_inventory(entity) { query_relations(entity, "contains") }
fn get_party(player) { query_relations(player, "party_member").sort_by(|r| r.data.slot) }
fn get_equipped(char) { query_relations(char, "equips") }
fn get_location(entity) { query_relations(entity, "in")[0].to }
fn get_contents(container) { query_relations_to(container, "in") }
```

#### State Machines

```rust
pub struct StateMachine {
    pub id: Uuid,
    pub current_state: String,
    pub states: Vec<String>,
    pub transitions: Vec<Transition>,
    pub on_enter: HashMap<String, String>,  // State → script
    pub on_exit: HashMap<String, String>,
}
```

```rhai
let sm = create_state_machine(entity_id, #{
    states: ["idle", "attacking", "stunned", "dead"],
    initial: "idle",
    transitions: [
        #{ from: "idle", to: "attacking", event: "attack_input" },
        #{ from: "attacking", to: "idle", event: "attack_complete" },
        #{ from: "*", to: "stunned", event: "take_hit", guard: "not_blocking" },
        #{ from: "stunned", to: "idle", event: "stun_timeout" },
        #{ from: "*", to: "dead", event: "hp_zero" },
    ],
    on_enter: #{
        attacking: "scripts/combat/start_attack.rhai",
        stunned: "scripts/combat/apply_stun.rhai",
        dead: "scripts/combat/handle_death.rhai"
    }
});

// Trigger transitions
emit_to(entity_id, "attack_input");
get_state(entity_id)  // Returns current state
```

#### Formulas (Computed Values)

```rust
pub struct Formula {
    pub expression: String,      // Rhai expression
    pub dependencies: Vec<String>, // What it reads from
    pub cached_value: Dynamic,
    pub dirty: bool,
}
```

```rhai
// Define a formula on a component
define_formula(entity_id, "stats.effective_attack",
    "base_attack + equipment_bonus + buff_bonus");

// Auto-recalculates when dependencies change
get_computed(entity_id, "stats.effective_attack")

// Complex formulas
define_formula(entity_id, "damage_dealt",
    "(effective_attack * move_power * stab_bonus * type_effectiveness * crit_multiplier) / defense");
```

#### Triggers (Event → Action)

```rhai
// Register reactive behavior
register_trigger("entity_damaged", fn(ctx) {
    let entity = get_entity(ctx.target_id);

    // Check for faint
    if entity.hp <= 0 {
        emit("entity_fainted", #{ entity_id: ctx.target_id });
    }

    // Check for rage buff
    if entity.hp < entity.max_hp * 0.25 {
        add_buff(ctx.target_id, "rage", #{ attack_bonus: 1.5 });
    }
});

// Conditional triggers
register_trigger("item_used", #{
    filter: fn(ctx) { ctx.item.type == "evolution_stone" },
    handler: fn(ctx) { try_evolve(ctx.target_id, ctx.item); }
});
```

### What This Means for "Modules"

**Previous approach** (game-shaped):
```
combat_module, inventory_module, party_module, narrative_module...
```

**New approach** (primitive-based):

```
vm-core primitives:
  - Entity, Component, Relation, Event, Query, Mutation

vm-logic primitives:
  - StateMachine, Formula, Constraint, Trigger, Timer, Random

vm-spatial primitives:
  - Grid, Position, Pathfinding, Range queries

vm-time primitives:
  - Tick, Timer, Schedule, Cooldown
```

**Games are just compositions:**

| Game | Primary Primitives Used |
|------|------------------------|
| Pokemon | Relation (party, types), StateMachine (battle), Formula (damage), Trigger (evolution) |
| FF1 | Same as Pokemon + Grid (dungeons) |
| Zelda | Grid (rooms), Trigger (puzzles), StateMachine (combat), Relation (items) |
| Match-3 | Grid, Query (matches), Trigger (cascade), Formula (score) |
| Tower Defense | Relation (path), Timer (spawn/move), Query (targeting), Formula (damage) |
| Dating Sim | Relation (affection), StateMachine (relationship stages), Trigger (events) |
| MUD | Grid (rooms), Relation (exits), Trigger (commands) |

### Revised Primitive/Module Structure

```
crates/
  vm-core/           # Entity, Component, Relation, Event, Query, Mutation
  vm-logic/          # StateMachine, Formula, Constraint, Trigger
  vm-spatial/        # Grid, Position, Pathfinding, Range
  vm-time/           # Tick, Timer, Schedule, Cooldown
  vm-random/         # Distributions, WeightedSelection, Shuffle
  vm-physics/        # Collision, Velocity, Bounce (for action games)
  vm-schema/         # Type definitions, validation
  vm-scripting/      # Rhai integration for all primitives
```

**Not:**
```
vm-combat/      # Too game-shaped
vm-inventory/   # Just relations!
vm-party/       # Just relations!
vm-narrative/   # StateMachine + Trigger!
```

## Migration Phases

### Phase 1: Schema Foundation
- Create `vm-schema` crate with schema types
- Implement TOML schema loader
- Define Blackwing entities as schemas (Ship→entity_type, etc.)

**Key files to create:**
- `crates/vm-schema/src/lib.rs`
- `crates/vm-schema/src/entity_schema.rs`
- `crates/vm-schema/src/component_schema.rs`
- `crates/vm-schema/src/loader.rs`

### Phase 2: Generic Entity/State
- Create `vm-core` with Entity, ComponentStore, ComponentData
- Implement generic `StateProvider` trait
- Port `MutationOverlay` to generic components

**Key files to modify/create:**
- `crates/vm-core/src/entity.rs` (new)
- `crates/vm-core/src/state/accessor.rs` (generalize from `bw-game/src/state/accessor.rs`)
- `crates/vm-core/src/state/overlay.rs` (adapt from `bw-game/src/state/overlay.rs`)

### Phase 3: Dynamic Events
- Replace `GameEventType` enum with `EventTypeId`
- Update EventRegistry and EventDispatcher
- Schema validation for event payloads

**Key files to modify:**
- `crates/bw-core/src/events/game_events.rs` → extract to `vm-core`
- `crates/bw-scripting/src/events/dispatcher.rs` → adapt

### Phase 4: Generic Script Bindings
- Create `query_entity()`, `get_component()`, `set_component()`, etc.
- Deprecate `query_ship()`, `modify_ship()` with compatibility shims
- Migrate Blackwing scripts to new API

**Key files:**
- `crates/vm-scripting/src/bindings/entity_api.rs` (new)
- `crates/vm-scripting/src/bindings/event_api.rs` (new)

### Phase 5: Plugin System & Runtime
- Implement `GamePlugin` trait and `PluginManager`
- Package Blackwing as a plugin
- Generic game loop and persistence

**Key files:**
- `crates/vm-core/src/plugin.rs` (new)
- `crates/vm-runtime/src/game_loop.rs` (generalize)

### Phase 6: Validation
- Create a simple second game (card game?) to prove genericity
- Performance benchmarks
- Documentation

## Preserved Infrastructure

These systems are already well-designed and need minimal changes:
- `bw-game/src/state/overlay.rs` - Transaction/overlay model
- `bw-scripting/src/coroutines/` - Coroutine scheduler
- `bw-scripting/src/validation.rs` - Script validation (extend for schemas)
- `bw-scripting-macros/` - RhaiDeserialize macro
- Permission system in accessor

## Critical Files Reference

| Current File | Action | New Location |
|--------------|--------|--------------|
| `bw-game/src/state/accessor.rs` | Generalize | `vm-core/src/state/accessor.rs` |
| `bw-game/src/state/overlay.rs` | Adapt | `vm-core/src/state/overlay.rs` |
| `bw-core/src/events/game_events.rs` | Replace | `vm-core/src/events/` |
| `bw-scripting/src/schema.rs` | Extend | `vm-schema/src/` |
| `bw-scripting/src/bindings/state_api.rs` | Replace | `vm-scripting/src/bindings/entity_api.rs` |
| `bw-core/src/models/*.rs` | Delete | Replaced by schemas |

## Backward Compatibility Strategy

During migration, provide shims:
```rust
// query_ship(id) -> query_entity(id) filtered to type "ship"
// modify_ship(id, changes) -> set_component calls for each field
```

Scripts can migrate incrementally while both APIs exist.

---

## Phase 1 Detailed Implementation (STARTING POINT)

### Step 1.1: Create vm-schema crate

```bash
cargo new --lib crates/vm-schema
```

**Files to create:**

1. `crates/vm-schema/src/lib.rs` - Module exports
2. `crates/vm-schema/src/types.rs` - Core type IDs
   ```rust
   pub struct EntityTypeId(pub String);
   pub struct ComponentTypeId(pub String);
   pub struct EventTypeId(pub String);
   pub struct EnumTypeId(pub String);
   ```
3. `crates/vm-schema/src/field.rs` - Field schema definitions
   ```rust
   pub enum FieldType {
       String, Float, Int, Bool, Uuid, Vec3,
       Array(Box<FieldType>),
       Map,
       Enum(EnumTypeId),  // Reference to schema-defined enum
   }
   pub struct FieldSchema {
       name: String,
       field_type: FieldType,
       required: bool,
       default: Option<Dynamic>,
       computed: bool,  // NEW: Is this derived from other fields?
       derivation: Option<String>,  // NEW: Rhai expression for computed fields
   }
   ```
4. `crates/vm-schema/src/enum_type.rs` - **NEW**: Schema-defined enums
   ```rust
   pub struct EnumTypeSchema {
       pub id: EnumTypeId,
       pub name: String,
       pub variants: Vec<String>,
   }
   ```
5. `crates/vm-schema/src/component.rs` - Component schemas (with derivations)
6. `crates/vm-schema/src/entity.rs` - Entity type schemas
7. `crates/vm-schema/src/event.rs` - Event schemas
8. `crates/vm-schema/src/loader.rs` - TOML loader
9. `crates/vm-schema/src/registry.rs` - Schema registry
10. `crates/vm-schema/src/validate.rs` - Runtime validation
11. `crates/vm-schema/src/derivation.rs` - **NEW**: Computed property evaluation

### Step 1.2: Create vm-core crate

```bash
cargo new --lib crates/vm-core
```

**Files to create:**

1. `crates/vm-core/src/lib.rs` - Module exports
2. `crates/vm-core/src/entity.rs` - Entity struct with dynamic components
3. `crates/vm-core/src/component.rs` - ComponentData with rhai::Map storage
4. `crates/vm-core/src/query.rs` - EntityQuery for filtering
5. `crates/vm-core/src/mutation.rs` - Generic mutation types
6. `crates/vm-core/src/state/mod.rs` - State module
7. `crates/vm-core/src/state/provider.rs` - StateProvider trait
8. `crates/vm-core/src/state/overlay.rs` - Adapted from bw-game
9. `crates/vm-core/src/state/store.rs` - In-memory entity storage

### Step 1.3: Define Blackwing schemas

Create `games/blackwing/schema/` directory with:

1. `entities.toml` - Ship, Player, Sector, Squadron as entity types
2. `components.toml` - Health, Transform, ShipStats, Cargo, etc.
3. `events.toml` - EntityDamaged, EntitySpawned, etc.

### Step 1.4: Integration test

Create a test that:
1. Loads Blackwing schemas
2. Creates entities via the generic API
3. Queries and mutates components
4. Validates against schemas

### Dependencies to add to Cargo.toml

```toml
[workspace.dependencies]
vm-schema = { path = "crates/vm-schema" }
vm-core = { path = "crates/vm-core" }
```

### Success Criteria for Phase 1

- [ ] Can define entity types, components, events in TOML
- [ ] Can define custom enum types in schema
- [ ] Can define computed/derived properties with Rhai expressions
- [ ] Can create Entity instances with ComponentData
- [ ] Schema validation catches invalid component data
- [ ] Computed properties auto-evaluate when dependencies change
- [ ] Generic StateProvider trait compiles
- [ ] Overlay system works with generic components
- [ ] Blackwing's Ship/Player/Sector representable as schemas
- [ ] Dating Sim's Character/Relationship/Location representable as schemas (validation test)
