use ensnano_design::Curve;
use ensnano_design::*;
use std::env;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;

use ultraviolet::{Rotor3, Vec3};

const HEIGHT_BETWEEN_HELICES_2D: f32 = 5.;

fn main() {
    env_logger::init();

    let mut args = env::args();
    if args.len() < 2 {
        println!("Usage: {} JSON-filename(s)", args.nth(0).unwrap());
        process::exit(0);
    } else {
        for f in args.skip(1) {
            println!(
                "Converting {}...",
                PathBuf::from(&f).file_stem().unwrap().to_string_lossy()
            );
            json_to_ens(Path::new(&f));
            println!(
                "Converting {}... DONE\n",
                PathBuf::from(&f).file_stem().unwrap().to_string_lossy()
            );
        }
    }
}

fn json_to_ens(path: &Path) {
    let mut design = Design::new();
    let mut helices = design.helices.make_mut();
    let mut helix_ids = Vec::new();
    let mut helices_length_forward = Vec::new();
    let mut helices_length_backward = Vec::new();

    let mut path_out = PathBuf::from(path);
    path_out.set_extension("ens");

    let s = std::fs::read_to_string(path).unwrap();
    let input: EmbeddedHelixStructre = serde_json::from_str(&s).unwrap();

    for cycle in input.cycles.iter() {
        let interpolators = cycle
            .iter()
            .map(|i| InterpolationDescriptor::Chebyshev {
                coeffs: input.chebyshev_coeffs[*i].to_vec(),
                interval: [0., 1.],
            })
            .collect();
        let desc = InterpolatedCurveDescriptor {
            curve: input.surface.curve.clone(),
            half_turns_count: input.surface.parameters.half_turns_count,
            curve_scale_factor: input.surface.parameters.scale,
            revolution_radius: input.surface.parameters.revolution_radius,
            interpolation: interpolators,
            chevyshev_smoothening: input.chebyshev_smoothening,
        };
        let mut cache = Default::default();
        let curve = InstanciatedCurveDescriptor::try_instanciate(Arc::new(
            CurveDescriptor::InterpolatedCurve(desc.clone()),
        ))
        .unwrap()
        .make_curve(&Parameters::GEARY_2014_DNA, &mut cache);
        let len = curve.length_by_descretisation(0., curve.geometry.t_max(), 10_000_000);
        println!("length = {len}");
        for i in 0..cycle.len() {
            //log::info!("HELIX {}", cycle[i]);
            for t in TS {
                let point = curve.geometry.position(*t + i as f64);
                log::info!("{t}:\t {}\t {}\t {}", point.x, point.y, point.z)
            }
        }
        let mut helix = Helix::new(Vec3::zero(), Rotor3::identity());
        let isometry = Isometry2::new(
            Vec2::unit_y() * HEIGHT_BETWEEN_HELICES_2D * helix_ids.len() as f32,
            Rotor2::identity(),
        );
        helix.isometry2d = Some(isometry);
        helix.curve = Some(Arc::new(
            ensnano_design::CurveDescriptor::InterpolatedCurve(desc),
        ));
        helix_ids.push(helices.push_helix(helix));
        helices_length_forward.push(curve.nb_points_forwards());
        helices_length_backward.push(curve.nb_points_backwards());
    }
    drop(helices);
    design.get_updated_grid_data();
    let mut helices = design.helices.make_mut();
    for h in helices.values_mut() {
        let mut isometry = h.isometry2d.unwrap();
        for i in h.additonal_isometries.iter_mut() {
            isometry.append_translation(
                Vec2::unit_y() * HEIGHT_BETWEEN_HELICES_2D * helix_ids.len() as f32,
            );
            i.additional_isometry = Some(isometry);
        }
    }

    drop(helices);

    for (len_idx, h_id) in helix_ids.iter().enumerate() {
        let len = helices_length_forward[len_idx];
        let forward_strand = Strand {
            cyclic: false,
            junctions: vec![],
            sequence: None,
            color: 0xeb4034,
            domains: vec![Domain::HelixDomain(HelixInterval {
                helix: *h_id,
                start: 0,
                end: len as isize,
                forward: true,
                sequence: None,
            })],
            name: None,
        };
        let len = helices_length_backward[len_idx];
        let backward_strand = Strand {
            cyclic: false,
            junctions: vec![],
            sequence: None,
            color: 0xeb4034,
            domains: vec![Domain::HelixDomain(HelixInterval {
                helix: *h_id,
                start: 0,
                end: len as isize,
                forward: false,
                sequence: None,
            })],
            name: None,
        };
        design.strands.push(forward_strand);
        design.strands.push(backward_strand);
    }

    use std::io::Write;
    let json_content = serde_json::to_string_pretty(&design).ok().unwrap();
    let mut f = std::fs::File::create(path_out).ok().unwrap();
    f.write_all(json_content.as_bytes()).unwrap();

    /*
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
        */
}

use serde::Deserialize;

#[derive(Deserialize)]
struct SurfaceParameters {
    revolution_radius: f64,
    scale: f64,
    half_turns_count: isize,
}

#[derive(Deserialize)]
struct TwistedRevolutionSurface {
    curve: CurveDescriptor2D,
    parameters: SurfaceParameters,
}

#[derive(Deserialize)]
struct EmbeddedHelixStructre {
    surface: TwistedRevolutionSurface,
    cycles: Vec<Vec<usize>>,
    nb_helices: usize,
    chebyshev_coeffs: Vec<Vec<f64>>,
    chebyshev_smoothening: f64,
}

const TS: &[f64] = &[
    0.,
    1e-4,
    1e-3,
    1e-2,
    0.1,
    0.25,
    0.5,
    0.75,
    0.9,
    1. - 1e-2,
    1. - 1e-3,
    1. - 1e-4,
    1.,
];
