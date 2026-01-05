// Shared utility functions for scripts
//
// Import with: import "utils" as utils;

/// Clamp a value between min and max.
fn clamp_value(val, min, max) {
    if val < min { min }
    else if val > max { max }
    else { val }
}

/// Linear interpolation.
fn lerp_value(a, b, t) {
    a + (b - a) * clamp_value(t, 0.0, 1.0)
}

/// Calculate percentage (0-100).
fn percentage(current, max) {
    if max <= 0.0 { 0.0 }
    else { (current / max) * 100.0 }
}

/// Check if a value is within range.
fn in_range(val, min, max) {
    val >= min && val <= max
}

/// Pick a random item from an array.
fn pick_random(arr) {
    if arr.len() == 0 { () }
    else { arr[rand_int(0, arr.len() - 1)] }
}

/// Weighted random selection.
/// Takes array of #{item, weight} maps.
fn weighted_pick(items) {
    let total = 0.0;
    for item in items {
        total += item["weight"];
    }

    let roll = rand_float(0.0, total);
    let cumulative = 0.0;

    for item in items {
        cumulative += item["weight"];
        if roll <= cumulative {
            return item["item"];
        }
    }

    // Fallback to last item
    items[items.len() - 1]["item"]
}

/// Format a number with sign (+/-).
fn format_signed(n) {
    if n > 0 { "+" + n }
    else { n.to_string() }
}
