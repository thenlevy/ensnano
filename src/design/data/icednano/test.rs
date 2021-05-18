/*
ENSnano, a 3d graphical application for DNA nanostructures.
    Copyright (C) 2021  Nicolas Levy <nicolaspierrelevy@gmail.com> and Nicolas Schabanel <nicolas.schabanel@ens-lyon.fr>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use super::*;

#[test]
fn sanitize_with_insertions() {
    let domains = vec![
        Domain::HelixDomain(HelixInterval {
            helix: 0,
            start: 0,
            end: 10,
            forward: true,
            sequence: None,
        }),
        Domain::Insertion(3),
        Domain::Insertion(5),
        Domain::HelixDomain(HelixInterval {
            helix: 1,
            start: 0,
            end: 10,
            forward: false,
            sequence: None,
        }),
    ];
    assert_sane_domains_non_cyclic(sanitize_domains(&domains, false).as_slice())
}

#[test]
fn sanitize_domains_scadnano() {
    let input = r##" {
  "version": "0.15.0",
  "grid": "square",
  "helices": [
    {"grid_position": [0, 0]},
    {"grid_position": [0, 1]}
  ],
  "strands": [
    {
      "circular": true,
      "color": "#57bb00",
      "domains": [
        {"helix": 0, "forward": true, "start": 8, "end": 16},
        {"loopout": 5},
        {"helix": 1, "forward": false, "start": 8, "end": 16}
      ]
    }
  ]
      }"##;
    let scadnano_design: super::super::scadnano::ScadnanoDesign =
        serde_json::from_str(&input).expect("Failed to parse scadnano input");
    let ensnano_design =
        Design::from_scadnano(&scadnano_design).expect("Could not convert to ensnano");
    assert_eq!(ensnano_design.strands.len(), 1);
    let strand = ensnano_design.strands.values().next().unwrap();
    assert_eq!(strand.domains.len(), 3);
    assert!(strand.cyclic);
    let sane_domains = sanitize_domains(strand.domains.as_slice(), true);
    assert_sane_domains_non_cyclic(sane_domains.as_slice());
    assert_sane_domains_cyclic(sane_domains.as_slice());
    let junctions = read_junctions(sane_domains.as_slice(), true);
    assert_eq!(
        junctions,
        vec![
            DomainJunction::Adjacent,
            DomainJunction::UnindentifiedXover,
            DomainJunction::UnindentifiedXover
        ]
    );
}

fn assert_sane_domains_non_cyclic(dom: &[Domain]) {
    let mut prev_insertion = false;
    for d in dom.iter() {
        if let Domain::Insertion(_) = d {
            if prev_insertion {
                panic!("Two successive Insertions in {:?}", dom);
            } else {
                prev_insertion = true;
            }
        } else {
            prev_insertion = false;
        }
    }
}

fn assert_sane_domains_cyclic(dom: &[Domain]) {
    if dom.len() >= 2 {
        if let Some(Domain::Insertion(_)) = dom.first() {
            if let Some(Domain::Insertion(_)) = dom.last() {
                panic!("First and last domains are both insertions in cyclic strand")
            }
        }
    }
}
