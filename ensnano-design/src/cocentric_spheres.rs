use ensnano_design::Curve;
use ensnano_design::*;
use std::sync::Arc;

const LEN_SCAFFOLD: usize = 8064;

const PARAMETERS: Parameters = Parameters::GEARY_2014_DNA;
const NANOTUBE_LENGTH: usize = 30;
const NB_NANOTUBE: usize = 10;
const DELTA_RADIUS: f64 = NANOTUBE_LENGTH as f64 * PARAMETERS.z_step as f64;

/// Volume exclusion radius
const H: f32 = PARAMETERS.helix_radius + PARAMETERS.inter_helix_gap / 2.;


use std::f64::consts::PI;
const MIN_DIAMETER: f64 = (2. * H as f64 * NB_NANOTUBE as f64) / PI;

const LOWER_BOUND_RADIUS: f64 = DELTA_RADIUS + 1.1 * MIN_DIAMETER / 2.;

fn main() {
    let mut lower_bound_radius = LOWER_BOUND_RADIUS;
    let mut upper_bound_raidus = 40.0;

    let mut found = false;

    let mut len_big_sphere = None;
    let mut len_small_sphere = None;

    while !found {
        let big_radius = (lower_bound_radius + upper_bound_raidus) / 2.;
        let big_desc = SphereLikeSpiralDescriptor {
            theta_0: 0.,
            radius: big_radius,
            minimum_diameter: Some(MIN_DIAMETER),
        };
        let small_desc = SphereLikeSpiralDescriptor {
            theta_0: 0.,
            radius: big_radius - DELTA_RADIUS,
            minimum_diameter: Some(MIN_DIAMETER),
        };
        println!("upper bound {upper_bound_raidus}");
        println!("lower bound {lower_bound_radius}");
        println!("radius {big_radius}");
        let mut cache = Default::default();
        let big_curve = InstanciatedCurveDescriptor::try_instanciate(Arc::new(
            CurveDescriptor::SphereLikeSpiral(big_desc),
        ))
        .unwrap()
        .make_curve(&PARAMETERS, &mut cache);

        let small_curve = InstanciatedCurveDescriptor::try_instanciate(Arc::new(
            CurveDescriptor::SphereLikeSpiral(small_desc),
        ))
        .unwrap()
        .make_curve(&PARAMETERS, &mut cache);

        let nb_points = 2 * big_curve.nb_points() + 2 * small_curve.nb_points() + 2 * NANOTUBE_LENGTH * NB_NANOTUBE;

        println!("nb point {nb_points}");
        println!("objective {LEN_SCAFFOLD}");

        if nb_points < LEN_SCAFFOLD {
            lower_bound_radius = big_radius;
        } else if nb_points > LEN_SCAFFOLD {
            upper_bound_raidus = big_radius;
        } else {
            len_big_sphere = Some(big_curve.nb_points());
            len_small_sphere = Some(small_curve.nb_points());
            found = true
        }
    }

    let big_radius = (lower_bound_radius + upper_bound_raidus) / 2.;

    let mut design = Design::new();
    let mut helices = design.helices.make_mut();

    helices.insert(0, Helix::new_sphere_like_spiral(big_radius, 0., Some(MIN_DIAMETER)));
    helices.insert(1, Helix::new_sphere_like_spiral(big_radius, PI, Some(MIN_DIAMETER)));
    helices.insert(2, Helix::new_sphere_like_spiral(big_radius - DELTA_RADIUS, 0., Some(MIN_DIAMETER)));
    helices.insert(3, Helix::new_sphere_like_spiral(big_radius - DELTA_RADIUS, PI, Some(MIN_DIAMETER)));

    drop(helices);

    for helix in [0, 1] {
        for forward in [true, false] {
            for (set, len) in [len_big_sphere.unwrap() as isize, len_small_sphere.unwrap() as isize].into_iter().enumerate() {
                let big_strand = Strand {
                    cyclic: false,
                    junctions: vec![],
                    sequence:  None,
                    color: 0xeb4034,
                    domains: vec![Domain::HelixDomain(HelixInterval {
                        helix: helix + 2 * set,
                        start: 0,
                        end: *len,
                        forward,
                        sequence: None,
                    })],
                    name: None,
                };
                design.strands.push(big_strand);
            }
        }
    }

    use std::io::Write;
    let json_content = serde_json::to_string_pretty(&design).ok().unwrap();
    let mut f = std::fs::File::create("two_spheres_hole.ens").ok().unwrap();
    f.write_all(json_content.as_bytes());
}
