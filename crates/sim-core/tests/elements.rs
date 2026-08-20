//! Element loading. Spec 3.2: definitions live in data, not code.

mod common;

use sim_core::elements::{DataError, State};
use sim_core::{ElementTable, Fixed};

#[test]
fn the_shipped_data_file_loads() {
    let table = common::table();
    assert_eq!(
        table.len(),
        13,
        "structure, sand, water, the wet sand they react into, the gold it presses into, \
         the residue left over, the burntResidue it burns into and the fuel that \
         compacts from, the two chassis elements a belt's own footprint can be filled \
         with, the moltenSand sand melts into, the steam water boils into, and the \
         glass molten sand sets as"
    );

    // Currency is matter like everything else, and exactly one element is money.
    let currencies: Vec<&str> = table
        .iter()
        .filter(|element| element.currency)
        .map(|element| element.name.as_str())
        .collect();
    assert_eq!(currencies, ["gold"]);

    let sand = table.get(table.id_of("sand").expect("sand")).expect("sand");
    assert_eq!(sand.state, State::Powder);
    assert_eq!(sand.density, 1600);
    assert_eq!(sand.color, [0xC8, 0xA2, 0x4A]);
    assert_eq!(sand.thermal_conductivity, Fixed::parse("0.27").unwrap());

    let water = table
        .get(table.id_of("water").expect("water"))
        .expect("water");
    assert_eq!(water.state, State::Liquid);
    assert_eq!(water.melting_point, 273);
    assert_eq!(water.boiling_point, 373);

    let structure = table.get(table.id_of("structure").expect("structure")).expect("structure");
    assert_eq!(structure.state, State::Solid);
}

/// Sand must out-weigh water or it will not sink through it, and the washer machine
/// the whole design is built around stops working.
#[test]
fn sand_is_denser_than_water() {
    let table = common::table();
    let sand = table.get(table.id_of("sand").unwrap()).unwrap();
    let water = table.get(table.id_of("water").unwrap()).unwrap();
    assert!(sand.density > water.density);
}

/// A vault sorts itself by density (spec 5.1): a nugget has to out-weigh what a press
/// leaves behind, or gold never separates from the residue sitting on top of it.
#[test]
fn gold_out_masses_the_residue_a_press_leaves_behind() {
    let table = common::table();
    let density = |name: &str| table.get(table.id_of(name).unwrap()).unwrap().density;
    assert!(density("gold") > density("residue"));
    assert!(density("residue") > density("wetSand"));
}

/// Pressing has to name a real element, or a machine could be configured to convert
/// matter into something the data file never defined.
#[test]
fn wet_sand_presses_into_a_declared_residue() {
    let table = common::table();
    let wet_sand = table.get(table.id_of("wetSand").unwrap()).unwrap();
    let residue = table.get(wet_sand.residue).expect("wetSand's residue should resolve");
    assert_eq!(residue.name, "residue");
}

/// The chain from residue to fuel (spec 5.3), which is deliberately not one mechanism
/// end to end: residue *burns* into burntResidue wherever it is hot enough, and
/// burntResidue is *compacted* into fuel by a machine. Both links still have to resolve
/// to a real element rather than an id pointed at nothing.
#[test]
fn residue_burns_and_then_compacts_all_the_way_to_fuel() {
    let table = common::table();
    let residue = table.get(table.id_of("residue").unwrap()).unwrap();
    let burnt = table
        .get(residue.burns_into)
        .expect("residue's burnsInto should resolve");
    assert_eq!(burnt.name, "burntResidue");

    assert_eq!(
        residue.refined_into,
        sim_core::EMPTY,
        "nothing refines residue on demand any more — heat is the only thing that \
         converts it, so a machine pointed at it should read as blocked"
    );

    let fuel = table
        .get(burnt.refined_into)
        .expect("burntResidue's refinedInto should resolve");
    assert_eq!(fuel.name, "fuel");
    assert_eq!(fuel.refined_into, sim_core::EMPTY, "the chain ends at fuel, for now");
}

/// Burning needs `burnsInto`, `ignitionPoint` and `flammability` to agree. Two of the
/// three default to "never", so a partial declaration describes something that silently
/// does nothing — which the loader rejects rather than shipping.
#[test]
fn burning_without_a_threshold_or_a_chance_is_rejected() {
    const BASE: &str = r##"{"elements":[
        {"id":1,"name":"ash","state":"powder","density":1,"melting_point":9000,
         "boiling_point":9001,"thermal_conductivity":0.1,"color":"#000000",
         "color_variance":0,"flammability":0.0,"hardness":0.0},
        {"id":2,"name":"straw","state":"powder","density":1,"melting_point":9000,
         "boiling_point":9001,"thermal_conductivity":0.1,"color":"#000000",
         "color_variance":0,"flammability":FLAM,"hardness":0.0,
         "burnsInto":"ash"IGNITION}
    ]}"##;

    let source = |flammability: &str, ignition: &str| {
        BASE.replace("FLAM", flammability).replace("IGNITION", ignition)
    };

    assert_eq!(
        ElementTable::from_json(&source("0.5", "")).unwrap_err(),
        DataError::BadField { field: "ignitionPoint" },
        "burning with no threshold to cross would never fire"
    );
    assert_eq!(
        ElementTable::from_json(&source("0.0", r#","ignitionPoint":800"#)).unwrap_err(),
        DataError::BadField { field: "flammability" },
        "burning with no chance per tick would never fire either"
    );
    assert!(ElementTable::from_json(&source("0.5", r#","ignitionPoint":800"#)).is_ok());
}

/// Ids are declared in the file, not derived from its order, so the table must be
/// indexed by the declared id.
#[test]
fn ids_come_from_the_file_not_the_order() {
    let table = ElementTable::from_json(
        r##"{"elements":[
            {"id":9,"name":"second","state":"liquid","density":1,"melting_point":0,
             "boiling_point":1,"thermal_conductivity":0.1,"color":"#010203",
             "color_variance":0,"flammability":0.0,"hardness":0.0},
            {"id":4,"name":"first","state":"solid","density":2,"melting_point":0,
             "boiling_point":1,"thermal_conductivity":0.1,"color":"#040506",
             "color_variance":0,"flammability":0.0,"hardness":0.0}
        ]}"##,
    )
    .expect("should load");

    assert_eq!(table.get(9).unwrap().name, "second");
    assert_eq!(table.get(4).unwrap().name, "first");
    assert!(table.get(0).is_none(), "id 0 is empty space");
    assert!(table.get(5).is_none(), "undeclared ids stay empty");
}

#[test]
fn rejects_bad_data() {
    let cases: [(&str, DataError); 4] = [
        (
            r##"{"elements":[{"id":0,"name":"x","state":"solid","density":1,"melting_point":0,"boiling_point":1,"thermal_conductivity":0.1,"color":"#000000","color_variance":0,"flammability":0.0,"hardness":0.0}]}"##,
            DataError::ReservedId,
        ),
        (
            r##"{"elements":[{"id":1,"name":"x","state":"plasma","density":1,"melting_point":0,"boiling_point":1,"thermal_conductivity":0.1,"color":"#000000","color_variance":0,"flammability":0.0,"hardness":0.0}]}"##,
            DataError::BadField { field: "state" },
        ),
        (
            r##"{"elements":[{"id":1,"name":"x","state":"solid","density":1,"melting_point":0,"boiling_point":1,"thermal_conductivity":0.1,"color":"red","color_variance":0,"flammability":0.0,"hardness":0.0}]}"##,
            DataError::BadColor,
        ),
        (
            r##"{"elements":[{"id":1,"name":"x","state":"solid","melting_point":0,"boiling_point":1,"thermal_conductivity":0.1,"color":"#000000","color_variance":0,"flammability":0.0,"hardness":0.0}]}"##,
            DataError::MissingField { field: "density" },
        ),
    ];

    for (source, expected) in cases {
        assert_eq!(ElementTable::from_json(source).unwrap_err(), expected);
    }
}

#[test]
fn rejects_duplicate_ids() {
    let source = r##"{"elements":[
        {"id":1,"name":"a","state":"solid","density":1,"melting_point":0,"boiling_point":1,"thermal_conductivity":0.1,"color":"#000000","color_variance":0,"flammability":0.0,"hardness":0.0},
        {"id":1,"name":"b","state":"solid","density":1,"melting_point":0,"boiling_point":1,"thermal_conductivity":0.1,"color":"#000000","color_variance":0,"flammability":0.0,"hardness":0.0}
    ]}"##;
    assert_eq!(
        ElementTable::from_json(source).unwrap_err(),
        DataError::DuplicateId(1)
    );
}

/// Reactions are data too (spec 3.2), and the shipped file defines the washing step
/// from spec 5.3's worked example.
#[test]
fn the_shipped_reactions_load() {
    let rules = common::rules();
    assert_eq!(rules.reactions.len(), 1);

    let sand = rules.elements.id_of("sand").expect("sand");
    let water = rules.elements.id_of("water").expect("water");
    let wet = rules.elements.id_of("wetSand").expect("wetSand");

    let (reaction, flipped) = rules
        .reactions
        .between(sand, water)
        .expect("sand and water should react");
    assert!(!flipped, "sand is the first reactant as written");
    assert_eq!(reaction.products, [wet, wet]);

    // The pair must be found from either side, or a reaction would only fire when the
    // reactants happened to be the right way round.
    let (_, flipped_back) = rules
        .reactions
        .between(water, sand)
        .expect("order should not matter");
    assert!(flipped_back);

    // Two reactants, two products: a reaction changes composition without changing how
    // many cells hold matter.
    assert_eq!(reaction.reactants.len(), reaction.products.len());
}

/// Wet sand must out-weigh both its ingredients or it would float back up through them.
#[test]
fn wet_sand_is_the_densest_of_the_three() {
    let table = common::table();
    let density = |name: &str| table.get(table.id_of(name).unwrap()).unwrap().density;
    assert!(density("wetSand") > density("sand"));
    assert!(density("wetSand") > density("water"));
}
