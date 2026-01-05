//! Declarative macros for reducing boilerplate in view types.
//!
//! These macros generate getter and setter methods for ShipView, PlayerView, etc.

/// Generate getter methods for ship views.
///
/// # Usage
///
/// ```ignore
/// ship_getters! {
///     // Simple f64 getter: field as f64, default 0.0
///     get_hull -> f64, hull, 0.0;
///     get_shields -> f64, shields, 0.0;
///
///     // String getter
///     get_name -> String, name.clone(), String::new();
///
///     // i64 getter from u32
///     get_cargo_capacity -> i64, cargo_capacity as i64, 0;
/// }
/// ```
#[macro_export]
macro_rules! ship_getters {
    // f64 getter with as f64 conversion
    (@impl $self:ident, get_f64, $field:ident) => {
        $crate::context::with_accessor(|accessor| {
            accessor.get_ship($self.id)
                .ok()
                .flatten()
                .map(|s| s.$field as f64)
                .unwrap_or(0.0)
        }).unwrap_or(0.0)
    };

    // String getter with clone
    (@impl $self:ident, get_string, $field:ident) => {
        $crate::context::with_accessor(|accessor| {
            accessor.get_ship($self.id)
                .ok()
                .flatten()
                .map(|s| s.$field.clone())
                .unwrap_or_default()
        }).unwrap_or_default()
    };

    // i64 getter from integer field
    (@impl $self:ident, get_i64, $field:ident) => {
        $crate::context::with_accessor(|accessor| {
            accessor.get_ship($self.id)
                .ok()
                .flatten()
                .map(|s| s.$field as i64)
                .unwrap_or(0)
        }).unwrap_or(0)
    };

    // bool getter
    (@impl $self:ident, get_bool, $field:ident) => {
        $crate::context::with_accessor(|accessor| {
            accessor.get_ship($self.id)
                .ok()
                .flatten()
                .map(|s| s.$field)
                .unwrap_or(false)
        }).unwrap_or(false)
    };

    // String getter for Uuid (to_string)
    (@impl $self:ident, get_uuid_string, $field:ident) => {
        $crate::context::with_accessor(|accessor| {
            accessor.get_ship($self.id)
                .ok()
                .flatten()
                .map(|s| s.$field.to_string())
                .unwrap_or_default()
        }).unwrap_or_default()
    };

    // String getter for Option<Uuid>
    (@impl $self:ident, get_option_uuid_string, $field:ident) => {
        $crate::context::with_accessor(|accessor| {
            accessor.get_ship($self.id)
                .ok()
                .flatten()
                .and_then(|s| s.$field)
                .map(|id| id.to_string())
                .unwrap_or_default()
        }).unwrap_or_default()
    };

    // Main entry: define multiple getters
    (
        $(
            $(#[$meta:meta])*
            $vis:vis fn $name:ident(&mut $self:ident) -> $ret:ty { $kind:ident, $field:ident }
        );* $(;)?
    ) => {
        $(
            $(#[$meta])*
            $vis fn $name(&mut $self) -> $ret {
                ship_getters!(@impl $self, $kind, $field)
            }
        )*
    };
}

/// Generate setter methods for ship views.
///
/// # Usage
///
/// ```ignore
/// ship_setters! {
///     // Clamped f32 setter (value clamped 0-100, stored as f32)
///     set_hull(value: f64) -> hull, clamp_f32;
///     set_shields(value: f64) -> shields, clamp_f32;
///
///     // Direct f32 setter (no clamping)
///     set_fuel(value: f64) -> fuel, f32;
/// }
/// ```
#[macro_export]
macro_rules! ship_setters {
    // Main entry: define multiple setters
    (
        $(
            $(#[$meta:meta])*
            $vis:vis fn $name:ident(&mut $self:ident, $value:ident: $vtype:ty) { clamp_f32, $field:ident }
        );* $(;)?
    ) => {
        $(
            $(#[$meta])*
            $vis fn $name(&mut $self, $value: $vtype) {
                let clamped = $value.clamp(0.0, 100.0) as f32;
                $crate::context::with_accessor(|accessor| {
                    let _ = accessor.modify_ship($self.id, $crate::state::ShipChanges {
                        $field: Some(clamped),
                        ..Default::default()
                    });
                });
            }
        )*
    };
}

/// Generate getter methods for player views.
#[macro_export]
macro_rules! player_getters {
    // i64 getter (from i32)
    (@impl $self:ident, get_i64, $field:ident) => {
        $crate::context::with_accessor(|accessor| {
            accessor.get_player($self.id)
                .ok()
                .flatten()
                .map(|p| p.$field as i64)
                .unwrap_or(0)
        }).unwrap_or(0)
    };

    // String getter
    (@impl $self:ident, get_string, $field:ident) => {
        $crate::context::with_accessor(|accessor| {
            accessor.get_player($self.id)
                .ok()
                .flatten()
                .map(|p| p.$field.clone())
                .unwrap_or_default()
        }).unwrap_or_default()
    };

    // bool getter
    (@impl $self:ident, get_bool, $field:ident) => {
        $crate::context::with_accessor(|accessor| {
            accessor.get_player($self.id)
                .ok()
                .flatten()
                .map(|p| p.$field)
                .unwrap_or(false)
        }).unwrap_or(false)
    };

    // String getter for Uuid
    (@impl $self:ident, get_uuid_string, $field:ident) => {
        $crate::context::with_accessor(|accessor| {
            accessor.get_player($self.id)
                .ok()
                .flatten()
                .map(|p| p.$field.to_string())
                .unwrap_or_default()
        }).unwrap_or_default()
    };

    // String getter for Option<Uuid>
    (@impl $self:ident, get_option_uuid_string, $field:ident) => {
        $crate::context::with_accessor(|accessor| {
            accessor.get_player($self.id)
                .ok()
                .flatten()
                .and_then(|p| p.$field)
                .map(|id| id.to_string())
                .unwrap_or_default()
        }).unwrap_or_default()
    };

    // Main entry
    (
        $(
            $(#[$meta:meta])*
            $vis:vis fn $name:ident(&mut $self:ident) -> $ret:ty { $kind:ident, $field:ident }
        );* $(;)?
    ) => {
        $(
            $(#[$meta])*
            $vis fn $name(&mut $self) -> $ret {
                player_getters!(@impl $self, $kind, $field)
            }
        )*
    };
}

/// Generate setter methods for player views.
#[macro_export]
macro_rules! player_setters {
    // i32 setter
    (
        $(
            $(#[$meta:meta])*
            $vis:vis fn $name:ident(&mut $self:ident, $value:ident: $vtype:ty) { i32, $field:ident }
        );* $(;)?
    ) => {
        $(
            $(#[$meta])*
            $vis fn $name(&mut $self, $value: $vtype) {
                $crate::context::with_accessor(|accessor| {
                    let _ = accessor.modify_player($self.id, $crate::state::PlayerChanges {
                        $field: Some($value as i32),
                        ..Default::default()
                    });
                });
            }
        )*
    };

    // i64 setter
    (
        $(
            $(#[$meta:meta])*
            $vis:vis fn $name:ident(&mut $self:ident, $value:ident: $vtype:ty) { i64, $field:ident }
        );* $(;)?
    ) => {
        $(
            $(#[$meta])*
            $vis fn $name(&mut $self, $value: $vtype) {
                $crate::context::with_accessor(|accessor| {
                    let _ = accessor.modify_player($self.id, $crate::state::PlayerChanges {
                        $field: Some($value),
                        ..Default::default()
                    });
                });
            }
        )*
    };
}
