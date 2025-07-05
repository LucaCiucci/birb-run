use makefile_lossless::Makefile;

pub fn foo() {
    let m = Makefile::from_reader(std::fs::File::open("Makefile").unwrap()).unwrap();

    println!("Rules:");
    for rule in m.rules() {
        println!("- rule: {}", rule);
        println!("  targets:");
        for target in rule.targets() {
            println!("  - {target}");
        }
        println!("  recipes:");
        for recipe in rule.recipes() {
            println!("  - {recipe}");
        }
    }
}
