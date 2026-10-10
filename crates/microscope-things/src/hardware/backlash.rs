//! Backlash compensation, shared by every stage driver (ADR-0018).
//!
//! A geared axis has play: when it reverses, the motor turns a little before
//! the carriage follows. Compensation makes every move end travelling the
//! same way, so the play is always taken up on the same side and the
//! carriage stops where it is sent.
//!
//! Each axis has a signed number of backlash steps: its size is enough to
//! take up the play, and its sign is the direction moves should end in. For
//! each axis, the compensator tracks how much of the play is taken up in
//! that direction: 1 when it all is, 0 when it is all on the other side.

use super::stage::Position;

/// Tracks how each axis's play lies, and plans moves that end with it taken
/// up in the preferred direction.
#[derive(Debug, Clone, PartialEq)]
pub struct BacklashCompensation {
    /// For each axis, the share of the play taken up in the preferred
    /// direction, from 0 to 1.
    engaged: [f64; 3],
}

impl Default for BacklashCompensation {
    /// Nothing is known about the play, so the first compensated move on
    /// each axis corrects.
    fn default() -> Self {
        Self { engaged: [0.0; 3] }
    }
}

impl BacklashCompensation {
    /// The moves that make `displacement` and end with the play taken up:
    /// the move itself when every moving axis ends engaged, or two moves
    /// otherwise. The first stops short of the target by the backlash on
    /// those axes; the second covers the rest in the preferred direction.
    pub fn plan(&self, displacement: Position, backlash: Position) -> Vec<Position> {
        let moves = displacement.to_array();
        let steps = backlash.to_array();
        let correction: [i64; 3] = std::array::from_fn(|axis| {
            let (d, b) = (moves[axis], steps[axis]);
            let needed = d != 0 && b != 0 && self.engaged[axis] + (d as f64 / b as f64) < 1.0;
            if needed { b } else { 0 }
        });
        if correction == [0; 3] {
            return vec![displacement];
        }
        let correction = Position::from_array(correction);
        vec![displacement - correction, correction]
    }

    /// Records a move the stage made.
    pub fn record(&mut self, displacement: Position, backlash: Position) {
        let moves = displacement.to_array();
        let steps = backlash.to_array();
        for axis in 0..3 {
            if steps[axis] != 0 {
                let change = moves[axis] as f64 / steps[axis] as f64;
                self.engaged[axis] = (self.engaged[axis] + change).clamp(0.0, 1.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BACKLASH: Position = Position::new(200, -100, 0);

    #[test]
    fn moves_correct_until_the_play_is_taken_up_in_the_preferred_direction() {
        let mut compensation = BacklashCompensation::default();
        let target = Position::new(50, 50, 50);
        // Nothing is known yet: x and y correct, z has no backlash.
        let moves = compensation.plan(target, BACKLASH);
        assert_eq!(moves, [Position::new(-150, 150, 50), BACKLASH]);
        for made in &moves {
            compensation.record(*made, BACKLASH);
        }
        // The play is taken up: moves the preferred ways need no correction.
        let onward = Position::new(50, -50, 50);
        assert_eq!(compensation.plan(onward, BACKLASH), [onward]);
        // y moves its preferred way (negative), x reverses and corrects.
        let back = Position::new(-10, -10, 0);
        assert_eq!(
            compensation.plan(back, BACKLASH),
            [Position::new(-210, -10, 0), Position::new(200, 0, 0)]
        );
        // A partial reversal leaves the play partly taken up.
        compensation.record(Position::new(-50, 0, 0), BACKLASH);
        assert_eq!(
            compensation.plan(Position::new(49, 0, 0), BACKLASH).len(),
            2
        );
        assert_eq!(
            compensation.plan(Position::new(50, 0, 0), BACKLASH).len(),
            1
        );
    }
}
