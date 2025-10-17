use phonon::sample_loader::SampleBank;

#[test]
fn test_sample_bank_loads_bd() {
    let mut bank = SampleBank::new();

    // Try to load bd sample
    let bd_sample = bank.get_sample("bd");

    assert!(bd_sample.is_some(), "BD sample should load");

    let sample = bd_sample.unwrap();
    println!("BD sample loaded: {} samples", sample.len());
    assert!(sample.len() > 0, "BD sample should have audio data");

    // Check that sample has actual audio (not all zeros)
    let non_zero = sample.iter().any(|&s| s.abs() > 0.0001);
    assert!(non_zero, "BD sample should contain non-zero audio");
}

#[test]
fn test_sample_bank_loads_multiple() {
    let mut bank = SampleBank::new();

    let bd = bank.get_sample("bd");
    let cp = bank.get_sample("cp");
    let hh = bank.get_sample("hh");

    assert!(bd.is_some(), "BD should load");
    assert!(cp.is_some(), "CP should load");
    assert!(hh.is_some(), "HH should load");

    println!("BD: {} samples", bd.as_ref().unwrap().len());
    println!("CP: {} samples", cp.as_ref().unwrap().len());
    println!("HH: {} samples", hh.as_ref().unwrap().len());
}
