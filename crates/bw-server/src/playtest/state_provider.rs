//! StateProvider implementation for PlaytestInstance
//!
//! Allows scripts to operate on playtest state using the same API as live state.

use uuid::Uuid;

use bw_core::models::{CombatStance, Position, Ship};
use bw_game::state::{
    MutationResult, PlayerSnapshot, SectorSnapshot, ShipSnapshot, StateProvider, StateMutation,
    EntityType,
};
use bw_shared::ServerMessage;

use super::instance::PlaytestInstance;

impl StateProvider for PlaytestInstance {
    fn get_ship(&self, ship_id: Uuid) -> Option<ShipSnapshot> {
        self.ships.get(&ship_id).map(|ship| {
            let faction_tag = ship.faction_id.and_then(|fid| {
                self.factions.get(&fid).map(|f| f.tag.clone())
            });
            ShipSnapshot::from_core(&ship, faction_tag)
        })
    }

    fn get_ships_in_sector(&self, sector_id: Uuid) -> Vec<ShipSnapshot> {
        self.sectors
            .get(&sector_id)
            .map(|sector| {
                sector
                    .ship_ids
                    .iter()
                    .filter_map(|entry| self.ships.get(entry.key()))
                    .map(|ship| {
                        let faction_tag = ship.faction_id.and_then(|fid| {
                            self.factions.get(&fid).map(|f| f.tag.clone())
                        });
                        ShipSnapshot::from_core(&ship, faction_tag)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn get_ships_in_range(
        &self,
        sector_id: Uuid,
        position: Position,
        range: f64,
    ) -> Vec<ShipSnapshot> {
        self.sectors
            .get(&sector_id)
            .map(|sector| {
                sector
                    .ship_ids
                    .iter()
                    .filter_map(|entry| self.ships.get(entry.key()))
                    .filter(|ship| ship.position.distance_to(&position) <= range)
                    .map(|ship| {
                        let faction_tag = ship.faction_id.and_then(|fid| {
                            self.factions.get(&fid).map(|f| f.tag.clone())
                        });
                        ShipSnapshot::from_core(&ship, faction_tag)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn get_player(&self, player_id: Uuid) -> Option<PlayerSnapshot> {
        self.player_data
            .get(&player_id)
            .map(|player| {
                let faction_tag = self.factions.get(&player.faction_id)
                    .map(|f| f.tag.clone())
                    .unwrap_or_default();
                let squadron_tag = player.squadron_id.and_then(|sid| {
                    self.squadrons.get(&sid).map(|s| s.tag.clone())
                });
                PlayerSnapshot::from_core(&player, faction_tag, squadron_tag, false)
            })
    }

    fn get_sector(&self, sector_id: Uuid) -> Option<SectorSnapshot> {
        self.sectors
            .get(&sector_id)
            .map(|si| SectorSnapshot::from_sector(&si.sector))
    }

    fn apply_mutations(&self, mutations: Vec<StateMutation>) -> Vec<MutationResult> {
        mutations
            .into_iter()
            .map(|m| self.apply_mutation(m))
            .collect()
    }
}

impl PlaytestInstance {
    /// Apply a single mutation to playtest state.
    fn apply_mutation(&self, mutation: StateMutation) -> MutationResult {
        match &mutation {
            StateMutation::ModifyShip { ship_id, changes } => {
                if let Some(mut ship) = self.ships.get_mut(ship_id) {
                    if let Some(hull) = changes.hull {
                        ship.hull_integrity = hull.clamp(0.0, 100.0);
                    }
                    if let Some(shields) = changes.shields {
                        ship.shield_strength = shields.clamp(0.0, 100.0);
                    }
                    if let Some(ammo) = changes.ammunition {
                        ship.resources.ammunition = ammo.clamp(0.0, 100.0);
                    }
                    if let Some(fuel) = changes.fuel {
                        ship.resources.fuel = fuel.clamp(0.0, 100.0);
                    }
                    if let Some(morale) = changes.morale {
                        ship.crew.morale = morale.clamp(0.0, 100.0);
                    }
                    if let Some(exp) = changes.experience {
                        ship.crew.experience = exp;
                    }
                    if let Some(ref pos) = changes.position {
                        ship.position = *pos;
                    }
                    if let Some(ref status_change) = changes.status {
                        ship.status = status_change.to_ship_status();
                    }
                    if let Some(ref stance_str) = changes.combat_stance {
                        ship.combat_stance = match stance_str.to_lowercase().as_str() {
                            "aggressive" => CombatStance::Aggressive,
                            "defensive" => CombatStance::Defensive,
                            "evasive" => CombatStance::Evasive,
                            _ => CombatStance::Balanced,
                        };
                    }
                    if let Some(ref locked) = changes.locked_target {
                        ship.locked_target = *locked;
                    }
                    if let Some(ref cargo_add) = changes.add_cargo {
                        ship.add_cargo(
                            cargo_add.cargo_type.clone(),
                            cargo_add.quantity,
                            cargo_add.purchase_price,
                        );
                    }
                    if let Some(ref cargo_remove) = changes.remove_cargo {
                        ship.remove_cargo(&cargo_remove.cargo_type, cargo_remove.quantity);
                    }
                    if let Some(ref upgrade_install) = changes.install_upgrade {
                        ship.install_upgrade(
                            upgrade_install.upgrade_id.clone(),
                            upgrade_install.slot.clone(),
                        );
                    }
                    if let Some(ref slot) = changes.remove_upgrade_slot {
                        ship.remove_upgrade(slot);
                    }
                    MutationResult::success(mutation)
                } else {
                    MutationResult::failure(mutation, "Ship not found")
                }
            }

            StateMutation::ModifyPlayer { player_id, changes } => {
                if let Some(mut player) = self.player_data.get_mut(player_id) {
                    if let Some(rep) = changes.reputation {
                        player.resources.reputation = rep;
                    }
                    if let Some(fame) = changes.fame {
                        player.resources.fame = fame;
                    }
                    if let Some(rep_delta) = changes.reputation_delta {
                        player.resources.apply_reputation_change(rep_delta);
                    }
                    if let Some(fame_delta) = changes.fame_delta {
                        player.resources.fame = (player.resources.fame + fame_delta).max(0);
                    }
                    if let Some(credits) = changes.credits {
                        player.credits = credits.max(0);
                    }
                    if let Some(credits_delta) = changes.credits_delta {
                        player.credits = (player.credits + credits_delta).max(0);
                    }
                    MutationResult::success(mutation)
                } else {
                    MutationResult::failure(mutation, "Player not found")
                }
            }

            StateMutation::SpawnShip { config } => {
                // Create the NPC ship
                let ship = Ship::new_npc_ship(
                    config.name.clone(),
                    config.ship_class,
                    config.sector_id,
                    config.position,
                    config.faction_id,
                );
                let ship_id = ship.id;

                // Add to ships map
                self.ships.insert(ship_id, ship);

                // Track as created in playtest
                self.created_ship_ids.insert(ship_id, ());

                // Add to sector
                if let Some(sector) = self.sectors.get(&config.sector_id) {
                    sector.ship_ids.insert(ship_id, ());
                }

                MutationResult::success_with_id(mutation, ship_id)
            }

            StateMutation::DestroyEntity { entity_id, entity_type } => {
                match entity_type {
                    EntityType::Ship => {
                        if let Some((_, ship)) = self.ships.remove(entity_id) {
                            // Remove from sector
                            if let Some(sector) = self.sectors.get(&ship.sector_id) {
                                sector.ship_ids.remove(entity_id);
                            }
                            // Track as deleted (unless it was created in playtest)
                            if !self.created_ship_ids.contains_key(entity_id) {
                                self.deleted_ship_ids.insert(*entity_id, ());
                            }
                            self.created_ship_ids.remove(entity_id);
                            MutationResult::success(mutation)
                        } else {
                            MutationResult::failure(mutation, "Ship not found")
                        }
                    }
                    EntityType::Mission => {
                        // Find and remove mission from any sector
                        for sector in self.sectors.iter() {
                            if sector.missions.remove(entity_id).is_some() {
                                return MutationResult::success(mutation);
                            }
                        }
                        MutationResult::failure(mutation, "Mission not found")
                    }
                    EntityType::Station => {
                        // Find and remove station (location) from any sector
                        for mut sector in self.sectors.iter_mut() {
                            let initial_len = sector.sector.locations.len();
                            sector.sector.locations.retain(|loc| loc.id != *entity_id);
                            if sector.sector.locations.len() < initial_len {
                                return MutationResult::success(mutation);
                            }
                        }
                        MutationResult::failure(mutation, "Station not found")
                    }
                    EntityType::Sector => {
                        if let Some((_, sector)) = self.sectors.remove(entity_id) {
                            // Move all ships in this sector to limbo
                            for ship_entry in sector.ship_ids.iter() {
                                if let Some(mut ship) = self.ships.get_mut(ship_entry.key()) {
                                    ship.sector_id = Uuid::nil();
                                }
                            }
                            MutationResult::success(mutation)
                        } else {
                            MutationResult::failure(mutation, "Sector not found")
                        }
                    }
                }
            }

            StateMutation::EmitEvent {
                event_type,
                data: _,
                actor_id,
                target_id,
            } => {
                // Log the event (full event dispatch would require more infrastructure)
                tracing::debug!(
                    playtest_id = %self.id,
                    event_type = %event_type,
                    actor = ?actor_id,
                    target = ?target_id,
                    "Playtest script emitted event"
                );
                MutationResult::success(mutation)
            }

            StateMutation::SendNotification {
                player_id,
                message,
                notification_type,
            } => {
                tracing::debug!(
                    playtest_id = %self.id,
                    player_id = %player_id,
                    notification_type = %notification_type,
                    message = %message,
                    "Playtest script queued notification"
                );
                // Send if player is connected to playtest
                if let Some(participant) = self.participants.get(&player_id) {
                    if let Some(ref conn) = participant.connection {
                        let msg = ServerMessage::Notification {
                            message: message.clone(),
                            notification_type: notification_type.clone(),
                        };
                        if let Err(e) = conn.try_send(msg) {
                            tracing::warn!(
                                playtest_id = %self.id,
                                player_id = %player_id,
                                error = %e,
                                "Failed to send notification to player"
                            );
                        }
                    }
                }
                MutationResult::success(mutation)
            }

            StateMutation::SendChoice {
                player_id,
                choice_id,
                description,
                choices,
            } => {
                tracing::debug!(
                    playtest_id = %self.id,
                    player_id = %player_id,
                    choice_id = %choice_id,
                    num_choices = choices.len(),
                    "Playtest script queued choice dialog"
                );
                // Send if player is connected to playtest
                if let Some(participant) = self.participants.get(&player_id) {
                    if let Some(ref conn) = participant.connection {
                        let msg = ServerMessage::ChoiceRequired {
                            choice_id: choice_id.clone(),
                            description: description.clone(),
                            choices: choices
                                .iter()
                                .map(|c| bw_shared::dto::ChoiceDto {
                                    id: c.id.clone(),
                                    text: c.text.clone(),
                                    is_available: c.is_available,
                                    requirement_text: c.requirement_text.clone(),
                                })
                                .collect(),
                        };
                        if let Err(e) = conn.try_send(msg) {
                            tracing::warn!(
                                playtest_id = %self.id,
                                player_id = %player_id,
                                choice_id = %choice_id,
                                error = %e,
                                "Failed to send choice dialog to player"
                            );
                        }
                    }
                }
                MutationResult::success(mutation)
            }

            StateMutation::BroadcastToSector {
                sector_id,
                message,
                notification_type,
            } => {
                tracing::debug!(
                    playtest_id = %self.id,
                    sector_id = %sector_id,
                    notification_type = %notification_type,
                    message = %message,
                    "Playtest script queued sector broadcast"
                );
                // Broadcast to all players in the sector
                if let Some(sector) = self.sectors.get(&sector_id) {
                    let msg = ServerMessage::Notification {
                        message: message.clone(),
                        notification_type: notification_type.clone(),
                    };
                    for conn in sector.connections.iter() {
                        if let Err(e) = conn.value().try_send(msg.clone()) {
                            tracing::warn!(
                                playtest_id = %self.id,
                                sector_id = %sector_id,
                                error = %e,
                                "Failed to broadcast to player in sector"
                            );
                        }
                    }
                }
                MutationResult::success(mutation)
            }
        }
    }
}
