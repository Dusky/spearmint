//! Element loading. Spec 3.2: definitions live in data, not code.

mod common;

use sim_core::elements::{DataError, State};
use sim_core::{ElementTable, Fixed};

#[test]
fn the_shipped_data_file_loads() {
    let table = common::table();
    assert_eq!(
        table.len(),
        4,
        "wall, sand, water, and the wet sand they react into"
    );

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

    let wall = table.get(table.id_of("wall").expect("wall")).expect("wall");
    assert_eq!(wall.state, State::Solid);
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
