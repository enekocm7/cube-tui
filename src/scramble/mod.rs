pub const fn generate_scramble(seed: usize) -> &'static str {
    const SCRAMBLES: [&str; 6] = [
        "R U R' U'",
        "F R U R' U' F'",
        "L U2 L' U2",
        "R2 U2 R2 U2",
        "F U R U' R' F'",
        "R U R' U R U2 R'",
    ];

    SCRAMBLES[seed % SCRAMBLES.len()]
}
