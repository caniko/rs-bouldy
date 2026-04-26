//! Compile-time smoke tests for `#[unreal_mod]`.

#[test]
fn unreal_mod_macro_smoke_tests() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/unit_mod_pass.rs");
    tests.compile_fail("tests/ui/non_unit_mod_fail.rs");
}
