use ensnano_design::Curve;
use ensnano_design::*;
use std::sync::Arc;

const LEN_SCAFFOLD: usize = 8064 / 2;

fn main() {
    let curve = CurveDescriptor2D::Ellipse {
        semi_minor_axis: 1f64.into(),
        semi_major_axis: 2f64.into(),
    };

    let mut lower_bound_radius = 10.0;
    let mut upper_bound_raidus = 40.0;

    let mut found = false;

    let mut parameters = Parameters::GEARY_2014_DNA;

    parameters.inter_helix_gap = 1.0;

    while !found {
        let big_radius = (lower_bound_radius + upper_bound_raidus) / 2.;
        let desc = TwistedTorusDescriptor {
            big_radius: big_radius.into(),
            curve: curve.clone(),
            helix_index_shift_per_turn: -5,
            number_of_helix_per_section: 18,
            initial_index_shift: 0,
            initial_curvilinear_abscissa: 0f64.into(),
            symetry_per_turn: 3,
        };
        println!("upper bound {upper_bound_raidus}");
        println!("lower bound {lower_bound_radius}");
        println!("radius {big_radius}");
        let mut cache = Default::default();
        let curve = InstanciatedCurveDescriptor::try_instanciate(Arc::new(
            CurveDescriptor::TwistedTorus(desc),
        ))
        .unwrap()
        .make_curve(&parameters, &mut cache);

        let nb_points = curve.nb_points();

        println!("nb point {nb_points}");
        println!("objective {LEN_SCAFFOLD}");

        if nb_points < LEN_SCAFFOLD {
            lower_bound_radius = big_radius;
        } else if nb_points > LEN_SCAFFOLD {
            upper_bound_raidus = big_radius;
        } else {
            found = true
        }
    }
}
