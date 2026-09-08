//! Screen layout and pointer-transition logic.
//!
//! Screen positions (`ScreenInfo::x/y`) are topology-canvas coordinates used to
//! select the neighbour in a movement direction. Pointer positions are local to
//! their current screen. When a pointer crosses onto a differently sized
//! screen, the perpendicular coordinate is mapped by edge percentage. Thus the
//! midpoint of a 1080-pixel edge lands at the midpoint of a 1440-pixel edge.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    config::ScreenPlacement,
    model::{ScreenId, ScreenInfo},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointerDelta {
    pub x: f64,
    pub y: f64,
}

impl PointerDelta {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointerPosition {
    pub screen_id: ScreenId,
    /// Native local coordinate on `screen_id`.
    pub x: f64,
    /// Native local coordinate on `screen_id`.
    pub y: f64,
}

impl PointerPosition {
    pub fn new(screen_id: impl Into<ScreenId>, x: f64, y: f64) -> Self {
        Self {
            screen_id: screen_id.into(),
            x,
            y,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenTransition {
    pub from_screen_id: ScreenId,
    pub to_screen_id: ScreenId,
    pub direction: Direction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointerMove {
    pub position: PointerPosition,
    pub transitions: Vec<ScreenTransition>,
    /// Directions in which the pointer encountered an outer topology edge.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_directions: Vec<Direction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutBounds {
    pub left: i64,
    pub top: i64,
    pub right: i64,
    pub bottom: i64,
}

impl LayoutBounds {
    pub const fn width(self) -> u64 {
        (self.right - self.left) as u64
    }

    pub const fn height(self) -> u64 {
        (self.bottom - self.top) as u64
    }
}

#[derive(Debug, Clone, Default)]
pub struct ScreenTopology {
    screens: Vec<ScreenInfo>,
}

/// Concise alias for callers that already live in the topology module's domain.
pub type Topology = ScreenTopology;

impl ScreenTopology {
    pub fn new(screens: Vec<ScreenInfo>) -> Result<Self, TopologyError> {
        validate_screens(&screens)?;
        Ok(Self { screens })
    }

    pub fn screens(&self) -> &[ScreenInfo] {
        &self.screens
    }

    pub fn into_screens(self) -> Vec<ScreenInfo> {
        self.screens
    }

    pub fn screen(&self, screen_id: &str) -> Option<&ScreenInfo> {
        self.screens.iter().find(|screen| screen.id == screen_id)
    }

    pub fn available_screen_count(&self) -> usize {
        self.screens
            .iter()
            .filter(|screen| screen.is_available())
            .count()
    }

    /// Adds a screen or replaces the existing entry with the same ID.
    pub fn upsert_screen(
        &mut self,
        screen: ScreenInfo,
    ) -> Result<Option<ScreenInfo>, TopologyError> {
        validate_screen(&screen)?;
        if let Some(index) = self
            .screens
            .iter()
            .position(|existing| existing.id == screen.id)
        {
            Ok(Some(std::mem::replace(&mut self.screens[index], screen)))
        } else {
            self.screens.push(screen);
            Ok(None)
        }
    }

    pub fn remove_screen(&mut self, screen_id: &str) -> Option<ScreenInfo> {
        let index = self
            .screens
            .iter()
            .position(|screen| screen.id == screen_id)?;
        Some(self.screens.remove(index))
    }

    /// Sets an absolute, draggable canvas position.
    pub fn set_screen_position(
        &mut self,
        screen_id: &str,
        x: i32,
        y: i32,
    ) -> Result<(), TopologyError> {
        let screen = self.screen_mut(screen_id)?;
        screen.x = x;
        screen.y = y;
        Ok(())
    }

    /// Applies a drag delta, reporting coordinate overflow instead of wrapping.
    pub fn drag_screen(
        &mut self,
        screen_id: &str,
        delta_x: i32,
        delta_y: i32,
    ) -> Result<(i32, i32), TopologyError> {
        let screen = self.screen_mut(screen_id)?;
        let x = screen
            .x
            .checked_add(delta_x)
            .ok_or_else(|| TopologyError::CoordinateOverflow(screen_id.into()))?;
        let y = screen
            .y
            .checked_add(delta_y)
            .ok_or_else(|| TopologyError::CoordinateOverflow(screen_id.into()))?;
        screen.x = x;
        screen.y = y;
        Ok((x, y))
    }

    pub fn set_screen_enabled(
        &mut self,
        screen_id: &str,
        enabled: bool,
    ) -> Result<(), TopologyError> {
        self.screen_mut(screen_id)?.enabled = enabled;
        Ok(())
    }

    /// Applies saved positions to currently discovered screens. Stale entries
    /// and entries whose owner does not match are ignored safely.
    pub fn apply_placements(&mut self, placements: &[ScreenPlacement]) -> usize {
        let mut applied = 0;
        for placement in placements {
            if let Some(screen) = self.screens.iter_mut().find(|screen| {
                screen.id == placement.screen_id
                    && screen.owner_device_id == placement.owner_device_id
            }) {
                screen.x = placement.x;
                screen.y = placement.y;
                screen.enabled = placement.enabled;
                applied += 1;
            }
        }
        applied
    }

    pub fn placements(&self) -> Vec<ScreenPlacement> {
        self.screens
            .iter()
            .map(|screen| ScreenPlacement {
                screen_id: screen.id.clone(),
                owner_device_id: screen.owner_device_id,
                x: screen.x,
                y: screen.y,
                enabled: screen.enabled,
            })
            .collect()
    }

    /// Snaps a moved screen to the nearest edge of another available screen.
    /// The perpendicular coordinate is clamped so the two edges overlap and a
    /// pointer can actually cross between them.
    pub fn snap_screen_to_nearest_edge(&mut self, screen_id: &str) -> Result<(), TopologyError> {
        let moved = self
            .screen(screen_id)
            .cloned()
            .ok_or_else(|| TopologyError::UnknownScreen(screen_id.into()))?;
        let requested_x = i64::from(moved.x);
        let requested_y = i64::from(moved.y);
        let moved_width = i64::from(moved.width);
        let moved_height = i64::from(moved.height);
        let mut best: Option<(i128, i64, i64)> = None;

        for other in self
            .screens
            .iter()
            .filter(|screen| screen.id != screen_id && screen.is_available())
        {
            let other_x = i64::from(other.x);
            let other_y = i64::from(other.y);
            let other_right = other_x + i64::from(other.width);
            let other_bottom = other_y + i64::from(other.height);
            let vertical =
                requested_y.clamp(other_y - moved_height + 1, other_bottom.saturating_sub(1));
            let horizontal =
                requested_x.clamp(other_x - moved_width + 1, other_right.saturating_sub(1));
            let candidates = [
                (other_x - moved_width, vertical),
                (other_right, vertical),
                (horizontal, other_y - moved_height),
                (horizontal, other_bottom),
            ];
            for (x, y) in candidates {
                let dx = i128::from(x - requested_x);
                let dy = i128::from(y - requested_y);
                let score = dx * dx + dy * dy;
                if best.is_none_or(|(best_score, _, _)| score < best_score) {
                    best = Some((score, x, y));
                }
            }
        }

        if let Some((_, x, y)) = best {
            let screen = self.screen_mut(screen_id)?;
            screen.x = x.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
            screen.y = y.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        }
        Ok(())
    }

    /// Keeps persisted/UI coordinates compact without changing relative layout.
    pub fn normalize_origin(&mut self) {
        let Some(min_x) = self.screens.iter().map(|screen| screen.x).min() else {
            return;
        };
        let Some(min_y) = self.screens.iter().map(|screen| screen.y).min() else {
            return;
        };
        for screen in &mut self.screens {
            screen.x = (i64::from(screen.x) - i64::from(min_x))
                .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
            screen.y = (i64::from(screen.y) - i64::from(min_y))
                .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        }
    }

    /// Returns the first available screen containing the global point.
    /// Disabled, offline, or zero-sized screens never participate in hit tests.
    pub fn hit_test(&self, global_x: f64, global_y: f64) -> Option<&ScreenInfo> {
        self.screens.iter().find(|screen| {
            screen.is_available() && screen.contains_global_point(global_x, global_y)
        })
    }

    pub fn local_to_global(&self, position: &PointerPosition) -> Result<(f64, f64), TopologyError> {
        let screen = self.active_screen(&position.screen_id)?;
        validate_local_position(screen, position.x, position.y)?;
        Ok((
            f64::from(screen.x) + position.x,
            f64::from(screen.y) + position.y,
        ))
    }

    pub fn global_to_local(&self, global_x: f64, global_y: f64) -> Option<PointerPosition> {
        let screen = self.hit_test(global_x, global_y)?;
        Some(PointerPosition {
            screen_id: screen.id.clone(),
            x: global_x - f64::from(screen.x),
            y: global_y - f64::from(screen.y),
        })
    }

    /// The smallest rectangle containing every available screen.
    pub fn layout_bounds(&self) -> Option<LayoutBounds> {
        let mut available = self.screens.iter().filter(|screen| screen.is_available());
        let first = available.next()?;
        let mut bounds = LayoutBounds {
            left: i64::from(first.x),
            top: i64::from(first.y),
            right: i64::from(first.x) + i64::from(first.width),
            bottom: i64::from(first.y) + i64::from(first.height),
        };
        for screen in available {
            bounds.left = bounds.left.min(i64::from(screen.x));
            bounds.top = bounds.top.min(i64::from(screen.y));
            bounds.right = bounds
                .right
                .max(i64::from(screen.x) + i64::from(screen.width));
            bounds.bottom = bounds
                .bottom
                .max(i64::from(screen.y) + i64::from(screen.height));
        }
        Some(bounds)
    }

    pub fn move_pointer_by(
        &self,
        screen_id: &str,
        local_x: f64,
        local_y: f64,
        delta_x: f64,
        delta_y: f64,
    ) -> Result<PointerMove, TopologyError> {
        self.move_by(
            &PointerPosition::new(screen_id, local_x, local_y),
            PointerDelta::new(delta_x, delta_y),
        )
    }

    /// Applies an input-device delta and crosses as many screens as necessary.
    ///
    /// Crossing uses the source edge fraction for the perpendicular destination
    /// coordinate, so movement is reversible across different resolutions. A
    /// missing neighbour clamps only the blocked axis; diagonal movement may
    /// still slide along the outer edge.
    pub fn move_by(
        &self,
        start: &PointerPosition,
        delta: PointerDelta,
    ) -> Result<PointerMove, TopologyError> {
        if !start.x.is_finite()
            || !start.y.is_finite()
            || !delta.x.is_finite()
            || !delta.y.is_finite()
        {
            return Err(TopologyError::NonFiniteCoordinate);
        }

        let initial_screen = self.active_screen(&start.screen_id)?;
        validate_local_position(initial_screen, start.x, start.y)?;

        let mut position = start.clone();
        let mut remaining = delta;
        let mut transitions = Vec::new();
        let mut blocked_directions = Vec::new();
        // Directional neighbour selection is monotonic for a single-axis move.
        // This larger bound also accommodates diagonal corner crossings while
        // protecting the runtime from malformed overlapping layouts.
        let transition_limit = self.available_screen_count().saturating_mul(8).max(8);

        for _ in 0..=transition_limit {
            if remaining.x == 0.0 && remaining.y == 0.0 {
                return Ok(PointerMove {
                    position,
                    transitions,
                    blocked_directions,
                });
            }

            let screen = self.active_screen(&position.screen_id)?;
            let width = f64::from(screen.width);
            let height = f64::from(screen.height);
            let target_x = position.x + remaining.x;
            let target_y = position.y + remaining.y;

            if is_local_coordinate(target_x, width) && is_local_coordinate(target_y, height) {
                position.x = target_x;
                position.y = target_y;
                return Ok(PointerMove {
                    position,
                    transitions,
                    blocked_directions,
                });
            }

            let horizontal_crossing = crossing_time(position.x, target_x, remaining.x, width);
            let vertical_crossing = crossing_time(position.y, target_y, remaining.y, height);
            let (time, direction) = choose_crossing(
                horizontal_crossing,
                vertical_crossing,
                remaining,
                width,
                height,
            )
            .ok_or(TopologyError::NonFiniteCoordinate)?;

            position.x += remaining.x * time;
            position.y += remaining.y * time;
            remaining.x *= 1.0 - time;
            remaining.y *= 1.0 - time;

            // Eliminate tiny floating point residue at an exact boundary.
            if remaining.x.abs() < 1.0e-12 {
                remaining.x = 0.0;
            }
            if remaining.y.abs() < 1.0e-12 {
                remaining.y = 0.0;
            }

            match direction {
                Direction::Left => position.x = 0.0,
                Direction::Right => position.x = width,
                Direction::Up => position.y = 0.0,
                Direction::Down => position.y = height,
            }

            let edge_fraction = match direction {
                Direction::Left | Direction::Right => (position.y / height).clamp(0.0, 1.0),
                Direction::Up | Direction::Down => (position.x / width).clamp(0.0, 1.0),
            };

            if let Some(next) = self.neighbour(screen, direction, edge_fraction) {
                let previous_screen_id = position.screen_id.clone();
                position.screen_id = next.id.clone();
                match direction {
                    Direction::Right => {
                        position.x = 0.0;
                        position.y = map_edge_fraction(edge_fraction, next.height);
                    }
                    Direction::Left => {
                        // The exclusive right boundary plus a negative residual
                        // gives the exact overshoot on the next iteration.
                        position.x = f64::from(next.width);
                        position.y = map_edge_fraction(edge_fraction, next.height);
                    }
                    Direction::Down => {
                        position.x = map_edge_fraction(edge_fraction, next.width);
                        position.y = 0.0;
                    }
                    Direction::Up => {
                        position.x = map_edge_fraction(edge_fraction, next.width);
                        position.y = f64::from(next.height);
                    }
                }
                transitions.push(ScreenTransition {
                    from_screen_id: previous_screen_id,
                    to_screen_id: next.id.clone(),
                    direction,
                });
            } else {
                if !blocked_directions.contains(&direction) {
                    blocked_directions.push(direction);
                }
                // Preserve the perpendicular residual so diagonal movement
                // slides along a wall rather than stopping entirely.
                match direction {
                    Direction::Left => {
                        position.x = 0.0;
                        remaining.x = 0.0;
                    }
                    Direction::Right => {
                        position.x = largest_coordinate_below(width);
                        remaining.x = 0.0;
                    }
                    Direction::Up => {
                        position.y = 0.0;
                        remaining.y = 0.0;
                    }
                    Direction::Down => {
                        position.y = largest_coordinate_below(height);
                        remaining.y = 0.0;
                    }
                }
            }
        }

        Err(TopologyError::TransitionLimitExceeded)
    }

    fn screen_mut(&mut self, screen_id: &str) -> Result<&mut ScreenInfo, TopologyError> {
        self.screens
            .iter_mut()
            .find(|screen| screen.id == screen_id)
            .ok_or_else(|| TopologyError::UnknownScreen(screen_id.into()))
    }

    fn active_screen(&self, screen_id: &str) -> Result<&ScreenInfo, TopologyError> {
        let screen = self
            .screen(screen_id)
            .ok_or_else(|| TopologyError::UnknownScreen(screen_id.into()))?;
        if !screen.is_available() {
            return Err(TopologyError::UnavailableScreen(screen_id.into()));
        }
        Ok(screen)
    }

    fn neighbour(
        &self,
        source: &ScreenInfo,
        direction: Direction,
        edge_fraction: f64,
    ) -> Option<&ScreenInfo> {
        let source_left = f64::from(source.x);
        let source_top = f64::from(source.y);
        let source_right = source_left + f64::from(source.width);
        let source_bottom = source_top + f64::from(source.height);
        let exit_perpendicular = match direction {
            Direction::Left | Direction::Right => {
                source_top + edge_fraction * f64::from(source.height)
            }
            Direction::Up | Direction::Down => {
                source_left + edge_fraction * f64::from(source.width)
            }
        };

        self.screens
            .iter()
            .filter(|candidate| candidate.id != source.id && candidate.is_available())
            .filter_map(|candidate| {
                let left = f64::from(candidate.x);
                let top = f64::from(candidate.y);
                let right = left + f64::from(candidate.width);
                let bottom = top + f64::from(candidate.height);
                let (is_directional, primary_gap, perpendicular_gap) = match direction {
                    Direction::Right => (
                        left >= source_right,
                        (left - source_right).max(0.0),
                        distance_to_interval(exit_perpendicular, top, bottom),
                    ),
                    Direction::Left => (
                        right <= source_left,
                        (source_left - right).max(0.0),
                        distance_to_interval(exit_perpendicular, top, bottom),
                    ),
                    Direction::Down => (
                        top >= source_bottom,
                        (top - source_bottom).max(0.0),
                        distance_to_interval(exit_perpendicular, left, right),
                    ),
                    Direction::Up => (
                        bottom <= source_top,
                        (source_top - bottom).max(0.0),
                        distance_to_interval(exit_perpendicular, left, right),
                    ),
                };
                // A screen can only be entered through the part of the exit
                // edge it actually overlaps. A merely diagonal screen must not
                // become a left/right neighbour just because its centre is on
                // that side of the source.
                (is_directional && perpendicular_gap == 0.0).then_some((candidate, primary_gap))
            })
            .min_by(|(left_screen, left_score), (right_screen, right_score)| {
                left_score
                    .total_cmp(right_score)
                    .then_with(|| left_screen.id.cmp(&right_screen.id))
            })
            .map(|(screen, _)| screen)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TopologyError {
    #[error("screen ID must not be empty")]
    EmptyScreenId,
    #[error("duplicate screen ID {0}")]
    DuplicateScreenId(String),
    #[error("screen {screen_id} has invalid dimensions {width}x{height}")]
    InvalidDimensions {
        screen_id: String,
        width: u32,
        height: u32,
    },
    #[error("screen {0} was not found")]
    UnknownScreen(String),
    #[error("screen {0} is disabled, offline, or zero-sized")]
    UnavailableScreen(String),
    #[error("screen coordinate overflow while dragging {0}")]
    CoordinateOverflow(String),
    #[error("pointer coordinate or delta is not finite")]
    NonFiniteCoordinate,
    #[error("pointer ({x}, {y}) is outside screen {screen_id}")]
    PointerOutsideScreen {
        screen_id: String,
        x: String,
        y: String,
    },
    #[error("screen transition limit exceeded; topology may contain pathological overlaps")]
    TransitionLimitExceeded,
}

fn validate_screens(screens: &[ScreenInfo]) -> Result<(), TopologyError> {
    let mut ids = HashSet::with_capacity(screens.len());
    for screen in screens {
        validate_screen(screen)?;
        if !ids.insert(screen.id.as_str()) {
            return Err(TopologyError::DuplicateScreenId(screen.id.clone()));
        }
    }
    Ok(())
}

fn validate_screen(screen: &ScreenInfo) -> Result<(), TopologyError> {
    if screen.id.trim().is_empty() {
        return Err(TopologyError::EmptyScreenId);
    }
    if screen.width == 0 || screen.height == 0 {
        return Err(TopologyError::InvalidDimensions {
            screen_id: screen.id.clone(),
            width: screen.width,
            height: screen.height,
        });
    }
    Ok(())
}

fn validate_local_position(screen: &ScreenInfo, x: f64, y: f64) -> Result<(), TopologyError> {
    if !x.is_finite() || !y.is_finite() {
        return Err(TopologyError::NonFiniteCoordinate);
    }
    if !is_local_coordinate(x, f64::from(screen.width))
        || !is_local_coordinate(y, f64::from(screen.height))
    {
        return Err(TopologyError::PointerOutsideScreen {
            screen_id: screen.id.clone(),
            x: x.to_string(),
            y: y.to_string(),
        });
    }
    Ok(())
}

fn is_local_coordinate(value: f64, extent: f64) -> bool {
    value >= 0.0 && value < extent
}

fn crossing_time(position: f64, target: f64, delta: f64, extent: f64) -> Option<(f64, bool)> {
    if delta > 0.0 && target >= extent {
        Some(((extent - position) / delta, true))
    } else if delta < 0.0 && target < 0.0 {
        Some(((0.0 - position) / delta, false))
    } else {
        None
    }
}

fn choose_crossing(
    horizontal: Option<(f64, bool)>,
    vertical: Option<(f64, bool)>,
    delta: PointerDelta,
    width: f64,
    height: f64,
) -> Option<(f64, Direction)> {
    match (horizontal, vertical) {
        (Some((time, positive)), None) => Some((
            time.clamp(0.0, 1.0),
            if positive {
                Direction::Right
            } else {
                Direction::Left
            },
        )),
        (None, Some((time, positive))) => Some((
            time.clamp(0.0, 1.0),
            if positive {
                Direction::Down
            } else {
                Direction::Up
            },
        )),
        (
            Some((horizontal_time, horizontal_positive)),
            Some((vertical_time, vertical_positive)),
        ) => {
            // At an exact corner, choose the axis with greater normalized
            // velocity. This makes the result independent of raw resolutions.
            let choose_horizontal = if (horizontal_time - vertical_time).abs() < 1.0e-12 {
                delta.x.abs() / width >= delta.y.abs() / height
            } else {
                horizontal_time < vertical_time
            };
            if choose_horizontal {
                Some((
                    horizontal_time.clamp(0.0, 1.0),
                    if horizontal_positive {
                        Direction::Right
                    } else {
                        Direction::Left
                    },
                ))
            } else {
                Some((
                    vertical_time.clamp(0.0, 1.0),
                    if vertical_positive {
                        Direction::Down
                    } else {
                        Direction::Up
                    },
                ))
            }
        }
        (None, None) => None,
    }
}

fn map_edge_fraction(fraction: f64, destination_extent: u32) -> f64 {
    let extent = f64::from(destination_extent);
    (fraction.clamp(0.0, 1.0) * extent).min(largest_coordinate_below(extent))
}

fn largest_coordinate_below(positive_extent: f64) -> f64 {
    debug_assert!(positive_extent.is_finite() && positive_extent > 0.0);
    f64::from_bits(positive_extent.to_bits() - 1)
}

fn distance_to_interval(value: f64, start: f64, end: f64) -> f64 {
    if value < start {
        start - value
    } else if value > end {
        value - end
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::OsKind;
    use uuid::Uuid;

    fn screen(id: &str, x: i32, y: i32, width: u32, height: u32, owner_byte: u8) -> ScreenInfo {
        ScreenInfo {
            id: id.into(),
            native_id: format!("native-{id}"),
            owner_device_id: Uuid::from_bytes([owner_byte; 16]),
            owner_name: if owner_byte == 1 { "Mac" } else { "PC" }.into(),
            name: id.into(),
            width,
            height,
            scale_factor: if owner_byte == 1 { 2.0 } else { 1.0 },
            x,
            y,
            primary: id == "left",
            enabled: true,
            online: true,
        }
    }

    fn two_different_resolutions() -> ScreenTopology {
        ScreenTopology::new(vec![
            screen("left", 0, 0, 1920, 1080, 1),
            screen("right", 1920, 0, 2560, 1440, 2),
        ])
        .unwrap()
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-8,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn hit_test_honours_negative_positions_half_open_edges_and_availability() {
        let mut disabled = screen("disabled", 100, 100, 50, 50, 3);
        disabled.enabled = false;
        let mut offline = screen("offline", 200, 100, 50, 50, 4);
        offline.online = false;
        let topology = ScreenTopology::new(vec![
            screen("negative", -1280, -100, 1280, 720, 1),
            screen("main", 0, 0, 1920, 1080, 2),
            disabled,
            offline,
        ])
        .unwrap();

        assert_eq!(topology.hit_test(-1280.0, -100.0).unwrap().id, "negative");
        assert_eq!(topology.hit_test(-0.001, 619.999).unwrap().id, "negative");
        assert_eq!(topology.hit_test(0.0, 0.0).unwrap().id, "main");
        assert!(topology.hit_test(1920.0, 500.0).is_none());
        // These points overlap `main`, which is first and available; moving the
        // disabled/offline screens away verifies exclusion unambiguously below.
        assert!(topology.hit_test(f64::NAN, 0.0).is_none());

        let topology = ScreenTopology::new(vec![
            {
                let mut value = screen("disabled", 3000, 0, 100, 100, 3);
                value.enabled = false;
                value
            },
            {
                let mut value = screen("offline", 3200, 0, 100, 100, 4);
                value.online = false;
                value
            },
        ])
        .unwrap();
        assert!(topology.hit_test(3050.0, 50.0).is_none());
        assert!(topology.hit_test(3250.0, 50.0).is_none());
    }

    #[test]
    fn dragging_enabling_and_persisted_placements_update_the_layout() {
        let mut topology = two_different_resolutions();
        assert_eq!(
            topology.drag_screen("right", -320, 180).unwrap(),
            (1600, 180)
        );
        topology.set_screen_position("left", -1920, 0).unwrap();
        topology.set_screen_enabled("right", false).unwrap();
        assert_eq!(topology.available_screen_count(), 1);
        assert!(topology.hit_test(1700.0, 300.0).is_none());

        let placements = topology.placements();
        let mut fresh = two_different_resolutions();
        assert_eq!(fresh.apply_placements(&placements), 2);
        assert_eq!(fresh.screen("left").unwrap().x, -1920);
        assert_eq!(fresh.screen("right").unwrap().y, 180);
        assert!(!fresh.screen("right").unwrap().enabled);
    }

    #[test]
    fn horizontal_crossing_maps_edge_percentage_across_resolutions() {
        let topology = two_different_resolutions();
        let moved = topology
            .move_pointer_by("left", 1915.0, 540.0, 10.0, 0.0)
            .unwrap();

        assert_eq!(moved.position.screen_id, "right");
        assert_close(moved.position.x, 5.0);
        assert_close(moved.position.y, 720.0);
        assert_eq!(
            moved.transitions,
            vec![ScreenTransition {
                from_screen_id: "left".into(),
                to_screen_id: "right".into(),
                direction: Direction::Right,
            }]
        );
        assert!(moved.blocked_directions.is_empty());
    }

    #[test]
    fn resolution_mapping_is_reversible() {
        let topology = two_different_resolutions();
        let moved = topology
            .move_pointer_by("right", 3.0, 720.0, -8.0, 0.0)
            .unwrap();

        assert_eq!(moved.position.screen_id, "left");
        assert_close(moved.position.x, 1915.0);
        assert_close(moved.position.y, 540.0);
        assert_eq!(moved.transitions[0].direction, Direction::Left);
    }

    #[test]
    fn vertical_crossing_maps_width_percentage() {
        let topology = ScreenTopology::new(vec![
            screen("top", 0, 0, 1920, 1080, 1),
            screen("bottom", 0, 1080, 1280, 1024, 2),
        ])
        .unwrap();
        let moved = topology
            .move_pointer_by("top", 960.0, 1070.0, 0.0, 15.0)
            .unwrap();

        assert_eq!(moved.position.screen_id, "bottom");
        assert_close(moved.position.x, 640.0);
        assert_close(moved.position.y, 5.0);
        assert_eq!(moved.transitions[0].direction, Direction::Down);
    }

    #[test]
    fn a_large_delta_can_cross_multiple_screens() {
        let topology = ScreenTopology::new(vec![
            screen("a", 0, 0, 100, 100, 1),
            screen("b", 100, 0, 100, 100, 2),
            screen("c", 200, 0, 100, 100, 3),
        ])
        .unwrap();
        let moved = topology
            .move_pointer_by("a", 90.0, 50.0, 250.0, 0.0)
            .unwrap();

        assert_eq!(moved.position.screen_id, "c");
        assert!(moved.position.x < 100.0 && moved.position.x > 99.0);
        assert_close(moved.position.y, 50.0);
        assert_eq!(moved.transitions.len(), 2);
        assert_eq!(moved.transitions[0].to_screen_id, "b");
        assert_eq!(moved.transitions[1].to_screen_id, "c");
        assert_eq!(moved.blocked_directions, vec![Direction::Right]);
    }

    #[test]
    fn disabled_screen_is_skipped_when_selecting_a_neighbour() {
        let mut nearest = screen("nearest", 100, 0, 100, 100, 2);
        nearest.enabled = false;
        let topology = ScreenTopology::new(vec![
            screen("source", 0, 0, 100, 100, 1),
            nearest,
            screen("farther", 250, 0, 100, 100, 3),
        ])
        .unwrap();
        let moved = topology
            .move_pointer_by("source", 95.0, 25.0, 10.0, 0.0)
            .unwrap();
        assert_eq!(moved.position.screen_id, "farther");
        assert_close(moved.position.x, 5.0);
        assert_close(moved.position.y, 25.0);
    }

    #[test]
    fn blocked_diagonal_delta_slides_along_the_outer_edge() {
        let topology = ScreenTopology::new(vec![screen("only", 0, 0, 100, 100, 1)]).unwrap();
        let moved = topology
            .move_pointer_by("only", 95.0, 50.0, 20.0, 30.0)
            .unwrap();

        assert_eq!(moved.position.screen_id, "only");
        assert!(moved.position.x < 100.0 && moved.position.x > 99.0);
        assert_close(moved.position.y, 80.0);
        assert_eq!(moved.blocked_directions, vec![Direction::Right]);
    }

    #[test]
    fn dragged_coordinates_determine_the_crossing_direction() {
        let mut topology = ScreenTopology::new(vec![
            screen("source", 0, 0, 100, 100, 1),
            screen("movable", 100, 0, 100, 100, 2),
        ])
        .unwrap();
        topology.set_screen_position("movable", -100, 0).unwrap();

        let moved = topology
            .move_pointer_by("source", 2.0, 50.0, -5.0, 0.0)
            .unwrap();
        assert_eq!(moved.position.screen_id, "movable");
        assert_close(moved.position.x, 97.0);
        assert_eq!(moved.transitions[0].direction, Direction::Left);
    }

    #[test]
    fn vertically_stacked_offset_screen_is_not_a_horizontal_neighbour() {
        let topology = ScreenTopology::new(vec![
            screen("windows-above", 0, 0, 150, 100, 2),
            screen("mac-below", 100, 100, 100, 100, 1),
        ])
        .unwrap();

        let moved_left = topology
            .move_pointer_by("mac-below", 2.0, 50.0, -5.0, 0.0)
            .unwrap();
        assert_eq!(moved_left.position.screen_id, "mac-below");
        assert_eq!(moved_left.blocked_directions, vec![Direction::Left]);

        let moved_up = topology
            .move_pointer_by("mac-below", 40.0, 2.0, 0.0, -5.0)
            .unwrap();
        assert_eq!(moved_up.position.screen_id, "windows-above");
        assert_eq!(moved_up.transitions[0].direction, Direction::Up);

        let diagonal = ScreenTopology::new(vec![
            screen("upper-left", -100, 0, 100, 100, 2),
            screen("lower-right", 0, 100, 100, 100, 1),
        ])
        .unwrap();
        let moved_left = diagonal
            .move_pointer_by("lower-right", 2.0, 50.0, -5.0, 0.0)
            .unwrap();
        assert_eq!(moved_left.position.screen_id, "lower-right");
        assert_eq!(moved_left.blocked_directions, vec![Direction::Left]);
    }

    #[test]
    fn dragged_screen_snaps_to_a_crossable_edge_and_normalizes() {
        let mut topology = ScreenTopology::new(vec![
            screen("fixed", 5000, 1000, 100, 100, 1),
            screen("movable", 5210, 1025, 80, 80, 2),
        ])
        .unwrap();

        topology.snap_screen_to_nearest_edge("movable").unwrap();
        topology.normalize_origin();

        let fixed = topology.screen("fixed").unwrap();
        let movable = topology.screen("movable").unwrap();
        assert_eq!((fixed.x, fixed.y), (0, 0));
        assert_eq!((movable.x, movable.y), (100, 25));
    }

    #[test]
    fn layout_bounds_exclude_unavailable_screens() {
        let mut disabled = screen("disabled", 5000, 5000, 100, 100, 3);
        disabled.enabled = false;
        let topology = ScreenTopology::new(vec![
            screen("a", -1280, -100, 1280, 720, 1),
            screen("b", 0, 0, 1920, 1080, 2),
            disabled,
        ])
        .unwrap();
        let bounds = topology.layout_bounds().unwrap();
        assert_eq!(bounds.left, -1280);
        assert_eq!(bounds.top, -100);
        assert_eq!(bounds.right, 1920);
        assert_eq!(bounds.bottom, 1080);
        assert_eq!(bounds.width(), 3200);
        assert_eq!(bounds.height(), 1180);
    }

    #[test]
    fn invalid_topologies_and_pointer_inputs_are_rejected() {
        let duplicate = screen("same", 100, 0, 100, 100, 2);
        assert!(matches!(
            ScreenTopology::new(vec![screen("same", 0, 0, 100, 100, 1), duplicate]),
            Err(TopologyError::DuplicateScreenId(id)) if id == "same"
        ));

        let invalid = screen("zero", 0, 0, 0, 100, 1);
        assert!(matches!(
            ScreenTopology::new(vec![invalid]),
            Err(TopologyError::InvalidDimensions { .. })
        ));

        let topology = two_different_resolutions();
        assert!(matches!(
            topology.move_pointer_by("left", 1920.0, 0.0, 1.0, 0.0),
            Err(TopologyError::PointerOutsideScreen { .. })
        ));
        assert_eq!(
            topology
                .move_pointer_by("left", 10.0, 10.0, f64::NAN, 0.0)
                .unwrap_err(),
            TopologyError::NonFiniteCoordinate
        );
    }

    #[test]
    fn pointer_and_transition_models_are_camel_case() {
        let topology = two_different_resolutions();
        let moved = topology
            .move_pointer_by("left", 1919.0, 100.0, 2.0, 0.0)
            .unwrap();
        let value = serde_json::to_value(moved).unwrap();
        assert_eq!(value["position"]["screenId"], "right");
        assert_eq!(value["transitions"][0]["fromScreenId"], "left");
        assert_eq!(value["transitions"][0]["direction"], "right");
        assert!(value.get("blockedDirections").is_none());
    }

    #[test]
    fn os_kind_import_remains_available_to_screen_consumers() {
        // A small compile-time guard that model/topology remain usable together.
        assert_eq!(
            serde_json::to_string(&OsKind::Windows).unwrap(),
            "\"windows\""
        );
    }
}
