use ensnano_design::Curve;
use ensnano_design::*;
use std::sync::Arc;

use ultraviolet::{Rotor3, Vec3};

fn main() {
    let mut design = Design::new();
    let mut helices = design.helices.make_mut();
    let mut helix_ids = Vec::new();
    let mut helices_length = Vec::new();

    let s = include_str!("../pentagonal_2x14.json");
    let input: EmbeddedHelixStructre = serde_json::from_str(s).unwrap();

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
        let len = curve.length_by_descretisation(0., input.nb_helices as f64 / 2., 100_000);
        println!("length = {len}");
        for i in 0..input.nb_helices / 2 {
            let len = curve.length_by_descretisation(i as f64, i as f64 + 1., 100_000);
            println!("length segment {i} = {len}");
        }
        let mut helix = Helix::new(Vec3::zero(), Rotor3::identity());
        helix.curve = Some(Arc::new(
            ensnano_design::CurveDescriptor::InterpolatedCurve(desc),
        ));
        helix_ids.push(helices.push_helix(helix));
        helices_length.push(curve.nb_points());
    }
    drop(helices);
    design.get_updated_grid_data();

    for (len_idx, h_id) in helix_ids.iter().enumerate() {
        let len = helices_length[len_idx];
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
    let mut f = std::fs::File::create("pentagonal.ens").ok().unwrap();
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
