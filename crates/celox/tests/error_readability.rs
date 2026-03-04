use celox::{Simulator, SimulatorBuilder};
use insta::assert_snapshot;

#[test]
fn test_combinational_loop_error_readability() {
    let code = r#"
        module Top (
            a: input logic,
            y: output logic
        ) {
            assign y = ~y & a;
        }
    "#;
    let res = SimulatorBuilder::new(code, "Top").build();

    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert_snapshot!(err);
}

#[test]
fn test_multiple_driver_error_readability() {
    let code = r#"
        module Top (
            a: input logic,
            y: output logic
        ) {
            assign y = a;
            assign y = ~a;
        }
    "#;
    let res = SimulatorBuilder::new(code, "Top").build();

    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert_snapshot!(err);
}

#[test]
fn test_multiple_errors_readability() {
    let code = r#"
        module Top (
            a: input logic,
            x: output logic,
            y: output logic
        ) {
            assign x = ~x & a;
            assign y = ~y & a;
        }
    "#;
    let res = SimulatorBuilder::new(code, "Top").build();

    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert_snapshot!(err);
}

#[test]
fn test_call_non_function_error_readability() {
    let code = r#"
        module Top (
            a: input logic,
            y: output logic
        ) {
            assign y = a();
        }
    "#;
    let res = SimulatorBuilder::new(code, "Top").build();

    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert_snapshot!(err);
}

#[test]
fn test_top_not_found_error_readability() {
    let code = r#"
        module Foo (a: input logic, b: output logic) {
            assign b = a;
        }
    "#;
    let res = Simulator::builder(code, "NonExistentTop").build();

    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert_snapshot!(err);
}

#[test]
fn test_combinational_loop_sir_error_readability() {
    let code = r#"
        module Top (a: input logic, o: output logic) {
            var x: logic;
            var y: logic;
            var z: logic;
            assign x = y;
            assign y = z;
            assign z = x;
            assign o = x;
        }
    "#;
    let res = Simulator::builder(code, "Top").build();

    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert_snapshot!(err);
}

#[test]
fn test_sv_module_unsupported_error_readability() {
    let code = r#"
        module Top (
            i_clk  : input  logic,
            i_rst_n: input  logic,
            i_d    : input  logic,
            o_d    : output logic,
        ) {
            inst u0: $sv::delay (
                i_clk,
                i_rst_n,
                i_d,
                o_d,
            );
        }
    "#;
    let res = Simulator::builder(code, "Top").build();

    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert_snapshot!(err);
}
