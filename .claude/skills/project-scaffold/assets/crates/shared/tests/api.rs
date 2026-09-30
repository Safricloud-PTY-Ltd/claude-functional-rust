//! Black-box tests of the `shared` crate's public API.

use shared::stub;

fn contract_with_parameters(width: u32, label: &str) -> u32 {
    stub!("contract_with_parameters", width, label)
}

fn contract_without_parameters() -> u32 {
    stub!("contract_without_parameters")
}

const fn const_contract(width: u32) -> u32 {
    stub!("const_contract", width)
}

#[test]
#[should_panic(expected = "not implemented: const_contract")]
fn stub_is_the_body_of_a_const_fn_and_panics_the_same_way() {
    let _ = const_contract(4);
}

#[test]
#[should_panic(expected = "not implemented: contract_with_parameters")]
fn stub_panics_with_not_implemented_and_the_contract_name() {
    let _ = contract_with_parameters(4, "label");
}

#[test]
#[should_panic(expected = "not implemented: contract_without_parameters")]
fn stub_accepts_a_name_with_no_parameters() {
    let _ = contract_without_parameters();
}
