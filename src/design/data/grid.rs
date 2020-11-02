
use super::icednano::Parameters;
use ultraviolet::Vec2;

pub trait GridDivision {

    fn origin_helix(parameters: &Parameters, x: isize, y: isize) -> Vec2;
    fn interpolate(parameters: &Parameters, x: f32, y: f32) -> (isize, isize);

}

pub struct SquareGrid;

impl GridDivision for SquareGrid {
    fn origin_helix(parameters: &Parameters, x: isize, y: isize) -> Vec2 {
        Vec2::new(
            x as f32 * (parameters.helix_radius * 2. + parameters.inter_helix_gap),
            y as f32 * (parameters.helix_radius * 2. + parameters.inter_helix_gap),
        )
    }

    fn interpolate(parameters: &Parameters, x: f32, y: f32) -> (isize, isize) {
        (
            (x / (parameters.helix_radius * 2. + parameters.inter_helix_gap)).round() as isize,
            (y / (parameters.helix_radius * 2. + parameters.inter_helix_gap)).round() as isize
        )
    }
}

pub struct HoneyComb; 

impl GridDivision for HoneyComb {
    fn origin_helix(parameters: &Parameters, x: isize, y: isize) -> Vec2 {
        let r = parameters.inter_helix_gap / 2. + parameters.helix_radius;
        let lower = 3. * r * y as f32;
        let upper = lower + r;
        Vec2::new(
            x as f32 * r * 3f32.sqrt(),
            if x % 2 == y % 2 {lower} else {upper},
        )
    }

    fn interpolate(parameters: &Parameters, x: f32, y: f32) -> (isize, isize) {
        let r = parameters.inter_helix_gap / 2. + parameters.helix_radius;
        let first_guess = (
            (x / (r * 3f32.sqrt())).round() as isize,
            (y / (3. * r)).floor() as isize
        );

        let mut ret = first_guess;
        let mut best_dist = (Self::origin_helix(parameters, first_guess.0, first_guess.1) - Vec2::new(x, y)).mag();
        for dx in [-1, 0, 1].iter() {
            for dy in [-1, 0, 1].iter() {
                let guess = (first_guess.0 + dx, first_guess.1 + dy);
                let dist = (Self::origin_helix(parameters, guess.0, guess.1) - Vec2::new(x, y)).mag();
                if dist < best_dist {
                    ret = guess;
                    best_dist = dist;
                }
            }
        }
        ret
    }

}
