use ensnano_design::grid::{GridDescriptor, GridTypeDescr, Hyperboloid};
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
const MIN_DIAMETER: f64 = (2. * H as f64 * NB_NANOTUBE as f64) / PI + 2. * H as f64;

const LOWER_BOUND_RADIUS: f64 = DELTA_RADIUS + 1.1 * MIN_DIAMETER / 2.;

fn main() {
    let mut lower_bound_radius = LOWER_BOUND_RADIUS;
    let mut upper_bound_raidus = 40.0;

    let mut found = false;

    let mut len_big_sphere = None;
    let mut len_small_sphere = None;

    while !found {
        let big_radius = (lower_bound_radius + upper_bound_raidus) / 2.;
        let small_radius = compute_small_radius(big_radius);
        let big_desc = SphereLikeSpiralDescriptor {
            theta_0: 0.,
            radius: big_radius,
            minimum_diameter: Some(MIN_DIAMETER),
        };
        let small_desc = SphereLikeSpiralDescriptor {
            theta_0: 0.,
            radius: small_radius,
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

        let nb_points = 2 * big_curve.nb_points()
            + 2 * small_curve.nb_points()
            + 2 * NANOTUBE_LENGTH * NB_NANOTUBE;

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
    let small_radius = compute_small_radius(big_radius);

    let mut design = Design::new();
    let mut helices = design.helices.make_mut();

    helices.insert(
        0,
        Helix::new_sphere_like_spiral(big_radius, 0., Some(MIN_DIAMETER)),
    );
    helices.insert(
        1,
        Helix::new_sphere_like_spiral(big_radius, PI, Some(MIN_DIAMETER)),
    );
    helices.insert(
        2,
        Helix::new_sphere_like_spiral(small_radius, 0., Some(MIN_DIAMETER)),
    );
    helices.insert(
        3,
        Helix::new_sphere_like_spiral(small_radius, PI, Some(MIN_DIAMETER)),
    );

    drop(helices);

    for helix in [0, 1] {
        for forward in [true, false] {
            for (set, len) in [
                len_big_sphere.unwrap() as isize,
                len_small_sphere.unwrap() as isize,
            ]
            .into_iter()
            .enumerate()
            {
                let big_strand = Strand {
                    cyclic: false,
                    junctions: vec![],
                    sequence: None,
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

    let (north, south) = nanotubes(big_radius);
    add_hyperboloid_helices(&mut design, north);
    add_hyperboloid_helices(&mut design, south);

    use std::io::Write;
    let json_content = serde_json::to_string_pretty(&design).ok().unwrap();
    let mut f = std::fs::File::create("two_spheres_hole.ens").ok().unwrap();
    f.write_all(json_content.as_bytes());
}

// let r1 be the radius of the big sphere, and phi1 be the smallest latitude on the big sphere.
// let r2 be the radius of the small sphere and phi2 the smallest latitude on the small sphere.
// let d be the smallest radius of both spheres
//
// We have h1 = r1 cos(phi1)
// By definition, h2 = h1 - delta
// Also, h2 = r2 cos(ph2)
// This gives r1 cos(phi1) - delta = r2 cos(phi2) (A)
//
// We also have d = r1 sin(phi1) = r2 sin(phi 2) (B)

// (B) gives phi1
fn get_phi1(big_radius: f64) -> f64 {
    (MIN_DIAMETER / (2. * big_radius)).asin()
}

// (A)^2 + (B)^2 gives r2
// r2^2 = r1^2 + delta^2 - 2delta*r1 cos(phi1)
// r2 = sqrt(r1^2 + delta^2 - 2delta * r1 cos(phi1)
fn compute_small_radius(big_radius: f64) -> f64 {
    let phi1 = get_phi1(big_radius);
    (big_radius.powi(2) + DELTA_RADIUS.powi(2) - 2. * DELTA_RADIUS * big_radius * phi1.cos()).sqrt()
}

fn nanotubes(big_radius: f64) -> (GridDescriptor, GridDescriptor) {
    use ultraviolet::{Rotor3, Vec3};

    let phi1 = get_phi1(big_radius);
    let north_position = (big_radius * phi1.cos()) as f32 * Vec3::unit_z();
    let hyperboloid = Hyperboloid {
        forced_radius: None,
        radius: 10,
        shift: 0.,
        nb_turn_per_100_nt: 0.,
        radius_shift: 0.,
        length: DELTA_RADIUS as f32,
    };
    let north_grid = GridDescriptor::hyperboloid(
        north_position,
        Rotor3::from_rotation_xz(-std::f32::consts::FRAC_PI_2),
        hyperboloid.clone(),
    );

    let south_position = -north_position;
    let south_grid = GridDescriptor::hyperboloid(
        south_position,
        Rotor3::from_rotation_xz(std::f32::consts::FRAC_PI_2),
        hyperboloid.clone(),
    );

    (north_grid, south_grid)
}

fn add_hyperboloid_helices(design: &mut Design, desc: GridDescriptor) {
    let mut grids = design.free_grids.make_mut();
    grids.push(desc);
    drop(grids);

    let grid_id = design.free_grids.keys().max().unwrap().clone();
    let grids = design.get_updated_grid_data();

    let grid = grids.grids.get(&grid_id.to_grid_id()).unwrap().clone();

    let mut helix_ids = vec![];
    for i in 0..NB_NANOTUBE {
        let helix = Helix::new_on_grid(&grid, i as isize, 0, grid_id.to_grid_id());
        let mut helices = design.helices.make_mut();
        helix_ids.push(helices.push_helix(helix));
    }

    for helix in helix_ids {
        for forward in [true, false] {
            {
                let big_strand = Strand {
                    cyclic: false,
                    junctions: vec![],
                    sequence: None,
                    color: 0xeb4034,
                    domains: vec![Domain::HelixDomain(HelixInterval {
                        helix: helix,
                        start: 0,
                        end: NANOTUBE_LENGTH as isize,
                        forward,
                        sequence: None,
                    })],
                    name: None,
                };
                design.strands.push(big_strand);
            }
        }
    }
}
