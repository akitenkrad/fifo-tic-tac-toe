use fifo_ttt_simulation::config::Cap;
use fifo_ttt_simulation::record::metric_name;

#[test]
fn metric_names_encode_finite_and_infinite_caps() {
    assert_eq!(metric_name("value", Cap::Finite(3), Cap::Inf), "value.kx3_koinf");
    assert_eq!(metric_name("n_states", Cap::Inf, Cap::Finite(1)), "n_states.kxinf_ko1");
}
